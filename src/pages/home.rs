use dioxus::prelude::*;

use crate::components::sources::SourceCatalogPanel;
use crate::i18n::use_texts;
use crate::models::{Category, Item, ItemPlacement, ProgressNodeKind};
use crate::routes::Route;
use crate::source_import;
use crate::state::{CategoryStore, ItemStore, SourceStore};
use crate::update_check::{self, StartUpdateCheck, UpdateCheckState};

/// Une œuvre (un placement précis) qui a des éléments nouveaux non vus.
#[derive(Clone, PartialEq)]
struct NewRelease {
    item_id: usize,
    category_id: usize,
    subcategory_id: usize,
    title: String,
    subcategory_name: String,
    image_url: Option<String>,
    /// Titre de chaque élément nouveau, suivi de sa date de sortie si la
    /// source en fournit une.
    new_titles: Vec<String>,
}

/// Une œuvre (un placement précis) commencée mais pas terminée.
#[derive(Clone, PartialEq)]
struct ContinueEntry {
    item_id: usize,
    category_id: usize,
    subcategory_id: usize,
    title: String,
    subcategory_name: String,
    image_url: Option<String>,
    done: usize,
    total: usize,
    next_title: String,
}

#[derive(Clone, Copy, PartialEq)]
enum HomeTab {
    NewReleases,
    Continue,
    Sources,
}

/// Une section de l'accueil par catégorie (dans l'ordre de la sidebar),
/// seulement celles où `make` a retenu au moins un placement. `make` reçoit
/// l'item, le placement et le nom de sa sous-catégorie.
fn group_by_category<T>(
    categories: &[Category],
    items: &[Item],
    make: impl Fn(&Item, &ItemPlacement, String) -> Option<T>,
) -> Vec<(usize, String, Vec<T>)> {
    categories
        .iter()
        .map(|category| {
            let entries: Vec<T> = items
                .iter()
                .flat_map(|item| {
                    item.placements.iter().filter(|p| p.category_id == category.id).filter_map(|p| {
                        let subcategory_name = category
                            .subcategories
                            .iter()
                            .find(|s| s.id == p.subcategory_id)
                            .map(|s| s.name.clone())
                            .unwrap_or_default();
                        make(item, p, subcategory_name)
                    })
                })
                .collect();
            (category.id, category.name.clone(), entries)
        })
        .filter(|(_, _, entries)| !entries.is_empty())
        .collect()
}

/// Accueil, en deux onglets groupés par catégorie :
/// - Nouveautés : vérification des nouveaux épisodes/chapitres des œuvres
///   en cours reliées à une source, et liste de ceux détectés depuis moins
///   de 14 jours et pas encore vus ;
/// - Continuer : œuvres commencées qui ont encore des éléments à faire ;
/// - Sources : catalogue de sources prêtes à l'emploi à ajouter.
#[component]
pub fn Home() -> Element {
    let i18n_text = use_texts();
    let item_store = use_context::<Signal<ItemStore>>();
    let category_store = use_context::<Signal<CategoryStore>>();
    let source_store = use_context::<Signal<SourceStore>>();
    let check_state = use_context::<Signal<UpdateCheckState>>();
    let checker = use_coroutine_handle::<StartUpdateCheck>();

    let (tracked, untracked) = update_check::count_ongoing(&item_store.read(), &source_store.read());
    let state = check_state();

    let now = source_import::now_secs();
    let mut tab = use_signal(|| HomeTab::NewReleases);

    let sections = group_by_category(&category_store.read().categories, &item_store.read().items, |item, p, subcategory_name| {
        let new_titles: Vec<String> = source_import::recent_new_leaves(&p.progress, now)
            .iter()
            .map(|n| match source_import::release_date(n) {
                Some(date) => format!("{} ({date})", n.title),
                None => n.title.clone(),
            })
            .collect();
        (!new_titles.is_empty()).then(|| NewRelease {
            item_id: item.id,
            category_id: p.category_id,
            subcategory_id: p.subcategory_id,
            title: item.title.clone(),
            subcategory_name,
            image_url: item.image_url.clone(),
            new_titles,
        })
    });

    // Commencées (au moins un élément coché) mais pas finies.
    let continue_sections = group_by_category(&category_store.read().categories, &item_store.read().items, |item, p, subcategory_name| {
        let leaves: Vec<_> = p.progress.iter().filter(|n| n.kind != ProgressNodeKind::Group).collect();
        let done = leaves.iter().filter(|n| n.is_done).count();
        let next = leaves.iter().find(|n| !n.is_done)?;
        (done > 0).then(|| ContinueEntry {
            item_id: item.id,
            category_id: p.category_id,
            subcategory_id: p.subcategory_id,
            title: item.title.clone(),
            subcategory_name,
            image_url: item.image_url.clone(),
            done,
            total: leaves.len(),
            next_title: next.title.clone(),
        })
    });

    let progress_percent = if state.total > 0 { state.done * 100 / state.total } else { 0 };

    rsx! {
        div { class: "home-view",
            div { class: "home-tabs",
                button {
                    r#type: "button",
                    class: if tab() == HomeTab::NewReleases { "home-tab active" } else { "home-tab" },
                    onclick: move |_| tab.set(HomeTab::NewReleases),
                    "{i18n_text.new_releases}"
                }
                button {
                    r#type: "button",
                    class: if tab() == HomeTab::Continue { "home-tab active" } else { "home-tab" },
                    onclick: move |_| tab.set(HomeTab::Continue),
                    "{i18n_text.continue_tab}"
                }
                button {
                    r#type: "button",
                    class: if tab() == HomeTab::Sources { "home-tab active" } else { "home-tab" },
                    onclick: move |_| tab.set(HomeTab::Sources),
                    "{i18n_text.sources_tab}"
                }
            }

            if tab() == HomeTab::Sources {
                SourceCatalogPanel {}
            } else if tab() == HomeTab::Continue {
                if continue_sections.is_empty() {
                    p { class: "empty-state", "{i18n_text.no_continue}" }
                }
                for (category_id , category_name , entries) in continue_sections {
                    section { key: "continue-{category_id}", class: "home-section",
                        h2 { class: "home-section-title", "{category_name}" }
                        div { class: "home-release-grid",
                            for entry in entries {
                                ContinueCard {
                                    key: "{entry.item_id}-{entry.category_id}-{entry.subcategory_id}",
                                    entry,
                                }
                            }
                        }
                    }
                }
            } else {
                div { class: "home-toolbar",
                    button {
                        r#type: "button",
                        class: "primary-btn",
                        disabled: state.running || tracked == 0,
                        onclick: move |_| checker.send(StartUpdateCheck),
                        if state.running {
                            {(i18n_text.checking_updates)(state.done, state.total)}
                        } else {
                            "{i18n_text.check_updates}"
                        }
                    }
                }

                if state.running {
                    div { class: "home-check-progress",
                        div { class: "home-check-bar",
                            div { class: "home-check-bar-fill", width: "{progress_percent}%" }
                        }
                        if let Some(current) = state.current.as_ref() {
                            span { class: "toolbar-hint", "{current}" }
                        }
                    }
                }

                if untracked > 0 {
                    p { class: "toolbar-hint", {(i18n_text.untracked_works)(untracked)} }
                }

                if !state.errors.is_empty() {
                    div { class: "home-check-errors",
                        p { class: "form-error", "{i18n_text.check_errors}" }
                        ul {
                            for (title , error) in state.errors.iter() {
                                li { class: "form-error", "{title} — " {i18n_text.search_error(error)} }
                            }
                        }
                    }
                }

                if sections.is_empty() {
                    p { class: "empty-state", "{i18n_text.no_new_releases}" }
                }

                for (category_id , category_name , releases) in sections {
                    section { key: "{category_id}", class: "home-section",
                        h2 { class: "home-section-title", "{category_name}" }
                        div { class: "home-release-grid",
                            for release in releases {
                                NewReleaseCard {
                                    key: "{release.item_id}-{release.category_id}-{release.subcategory_id}",
                                    release,
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Nombre de titres de nouveautés affichés avant de résumer par "…".
const SHOWN_TITLES: usize = 3;

#[component]
fn NewReleaseCard(release: NewRelease) -> Element {
    let i18n_text = use_texts();
    let mut item_store = use_context::<Signal<ItemStore>>();
    let NewRelease { item_id, category_id, subcategory_id, .. } = release;

    let image_style = release
        .image_url
        .as_ref()
        .map(|url| format!("background-image: url('{url}');"))
        .unwrap_or_default();
    let mut preview = release.new_titles.iter().take(SHOWN_TITLES).cloned().collect::<Vec<_>>().join(", ");
    if release.new_titles.len() > SHOWN_TITLES {
        preview.push_str(", …");
    }

    rsx! {
        div { class: "home-release",
            Link {
                class: "home-release-link",
                to: Route::ItemDetail { id: item_id, category_id, subcategory_id },
                div { class: "home-release-image", style: "{image_style}" }
                div { class: "home-release-body",
                    span { class: "home-release-subcategory", "{release.subcategory_name}" }
                    span { class: "home-release-title", "{release.title}" }
                    span { class: "home-release-count", {(i18n_text.new_count)(release.new_titles.len())} }
                    span { class: "home-release-preview", "{preview}" }
                }
            }
            button {
                r#type: "button",
                class: "home-release-seen",
                onclick: move |_| item_store.write().clear_new_flags(item_id, category_id, subcategory_id),
                "{i18n_text.mark_as_seen}"
            }
        }
    }
}

#[component]
fn ContinueCard(entry: ContinueEntry) -> Element {
    let i18n_text = use_texts();
    let ContinueEntry { item_id, category_id, subcategory_id, done, total, .. } = entry;

    let image_style = entry
        .image_url
        .as_ref()
        .map(|url| format!("background-image: url('{url}');"))
        .unwrap_or_default();
    let percent = done * 100 / total;

    rsx! {
        div { class: "home-release",
            Link {
                class: "home-release-link",
                to: Route::ItemDetail { id: item_id, category_id, subcategory_id },
                div { class: "home-release-image", style: "{image_style}" }
                div { class: "home-release-body",
                    span { class: "home-release-subcategory", "{entry.subcategory_name}" }
                    span { class: "home-release-title", "{entry.title}" }
                    div { class: "home-continue-progress",
                        div { class: "home-check-bar home-continue-bar",
                            div { class: "home-check-bar-fill", width: "{percent}%" }
                        }
                        span { class: "home-continue-count", {(i18n_text.progress_count)(done, total)} }
                    }
                    span { class: "home-release-preview", {(i18n_text.next_up)(&entry.next_title)} }
                }
            }
        }
    }
}
