use dioxus::prelude::*;

use crate::components::sources::SourceCatalogPanel;
use crate::i18n::{use_texts, Language, Texts};
use crate::models::{entry_placement, Category, Item, ItemPlacement, ItemStatus, ProgressMode, ProgressNode};
use crate::routes::Route;
use crate::source_import;
use crate::state::{CategoryStore, ItemStore, SourceStore, UniverseStore};
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

/// Une œuvre (un placement précis) en cours ou en pause.
#[derive(Clone, PartialEq)]
struct ContinueEntry {
    item_id: usize,
    category_id: usize,
    subcategory_id: usize,
    title: String,
    subcategory_name: String,
    image_url: Option<String>,
    /// Remplissage de la barre (en %), s'il y a une mesure ou une liste.
    bar: Option<f64>,
    /// Avancement écrit (« 3 / 12 », « 87 % », « 64,5 heures »).
    count: Option<String>,
    /// Où reprendre : l'élément commencé et sa position, sinon le premier
    /// élément pas encore coché.
    next_line: Option<String>,
    on_hold: bool,
}

/// Avancement d'une œuvre en cours : la mesure en mode pourcentage ou
/// compteur, sinon les éléments cochés de la liste.
fn progress_summary(i18n_text: &Texts, placement: &ItemPlacement, leaves: &[&ProgressNode]) -> (Option<f64>, Option<String>) {
    let value = placement.progress_value.unwrap_or(0.0);
    match placement.progress_mode {
        ProgressMode::Percent => (Some(value.clamp(0.0, 100.0)), Some((i18n_text.percent)(value))),
        ProgressMode::Counter => {
            let unit = &placement.counter.unit;
            let count = match placement.counter.target {
                Some(target) => format!("{} / {} {unit}", (i18n_text.number)(value), (i18n_text.number)(target)),
                None => format!("{} {unit}", (i18n_text.number)(value)),
            };
            (placement.measure_percent(), Some(count.trim().to_string()))
        }
        ProgressMode::List if !leaves.is_empty() => {
            let done = leaves.iter().filter(|n| n.is_done).count();
            (Some((done * 100 / leaves.len()) as f64), Some((i18n_text.progress_count)(done, leaves.len())))
        }
        _ => (None, None),
    }
}

/// Un univers commencé qui a encore une œuvre à voir.
#[derive(Clone, PartialEq)]
struct UniverseInProgress {
    universe_id: usize,
    name: String,
    done: usize,
    total: usize,
    next_title: String,
    image_url: Option<String>,
}

/// Univers en cours : au moins une œuvre commencée ou terminée, et une
/// prochaine œuvre à voir dans l'ordre affiché.
fn universes_in_progress(universes: &UniverseStore, items: &[Item]) -> Vec<UniverseInProgress> {
    universes
        .universes
        .iter()
        .filter_map(|universe| {
            let progress = universe.progress(items);
            let next = progress.next.filter(|_| progress.started)?;
            let (item, _) = entry_placement(&next, items)?;
            Some(UniverseInProgress {
                universe_id: universe.id,
                name: universe.name.clone(),
                done: progress.done,
                total: progress.total,
                next_title: item.title.clone(),
                image_url: item.image_url.clone(),
            })
        })
        .collect()
}

/// Ligne « où reprendre » d'une œuvre en cours : la position de la fiche
/// simple, ou le premier élément commencé (avec une position « reprendre
/// à »), ou à défaut le premier élément pas encore coché.
fn next_line(i18n_text: &Texts, placement: &ItemPlacement, leaves: &[&ProgressNode]) -> Option<String> {
    if let Some(position) = &placement.resume_at {
        return Some((i18n_text.resume_at)(position));
    }
    let pending = || leaves.iter().filter(|n| !n.is_done);
    if let Some((node, position)) = pending().find_map(|n| n.resume_at.as_ref().map(|p| (n, p))) {
        return Some((i18n_text.resume_item_at)(&node.title, position));
    }
    pending().next().map(|n| (i18n_text.next_up)(&n.title))
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
    language: Language,
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
                            .map(|s| s.display_name(language).to_string())
                            .unwrap_or_default();
                        make(item, p, subcategory_name)
                    })
                })
                .collect();
            (category.id, category.display_name(language).to_string(), entries)
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
    let language = use_context::<Memo<Language>>()();
    let item_store = use_context::<Signal<ItemStore>>();
    let category_store = use_context::<Signal<CategoryStore>>();
    let source_store = use_context::<Signal<SourceStore>>();
    let universe_store = use_context::<Signal<UniverseStore>>();
    let check_state = use_context::<Signal<UpdateCheckState>>();
    let checker = use_coroutine_handle::<StartUpdateCheck>();

    let (tracked, untracked) = update_check::count_ongoing(&item_store.read(), &source_store.read());
    let state = check_state();

    let now = source_import::now_secs();
    let mut tab = use_signal(|| HomeTab::NewReleases);

    let sections = group_by_category(&category_store.read().categories, &item_store.read().items, language, |item, p, subcategory_name| {
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

    // En cours, puis en pause (chaque catégorie les range dans cet ordre).
    let mut continue_sections = group_by_category(&category_store.read().categories, &item_store.read().items, language, |item, p, subcategory_name| {
        if !matches!(p.status, ItemStatus::InProgress | ItemStatus::OnHold) {
            return None;
        }
        let leaves: Vec<_> = p.leaves().collect();
        let (bar, count) = progress_summary(i18n_text, p, &leaves);
        Some(ContinueEntry {
            item_id: item.id,
            category_id: p.category_id,
            subcategory_id: p.subcategory_id,
            title: item.title.clone(),
            subcategory_name,
            image_url: item.image_url.clone(),
            bar,
            count,
            next_line: next_line(i18n_text, p, &leaves),
            on_hold: p.status == ItemStatus::OnHold,
        })
    });
    for (_, _, entries) in continue_sections.iter_mut() {
        entries.sort_by_key(|entry| entry.on_hold);
    }
    let universes = universes_in_progress(&universe_store.read(), &item_store.read().items);

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
                if continue_sections.is_empty() && universes.is_empty() {
                    p { class: "empty-state", "{i18n_text.no_continue}" }
                }
                if !universes.is_empty() {
                    section { class: "home-section",
                        h2 { class: "home-section-title", "{i18n_text.universes_in_progress}" }
                        div { class: "home-release-grid",
                            for universe in universes {
                                UniverseCard { key: "universe-{universe.universe_id}", universe }
                            }
                        }
                    }
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

/// Carte d'un univers en cours : prochaine œuvre (image et titre) et
/// avancement ; mène à la page de l'univers.
#[component]
fn UniverseCard(universe: UniverseInProgress) -> Element {
    let i18n_text = use_texts();
    let image_style = universe
        .image_url
        .as_ref()
        .map(|url| format!("background-image: url('{url}');"))
        .unwrap_or_default();
    let percent = universe.done * 100 / universe.total.max(1);

    rsx! {
        div { class: "home-release",
            Link {
                class: "home-release-link",
                to: Route::UniverseView { id: universe.universe_id },
                div { class: "home-release-image", style: "{image_style}" }
                div { class: "home-release-body",
                    span { class: "home-release-subcategory", "{i18n_text.universes}" }
                    span { class: "home-release-title", "{universe.name}" }
                    div { class: "home-continue-progress",
                        div { class: "home-check-bar home-continue-bar",
                            div { class: "home-check-bar-fill", width: "{percent}%" }
                        }
                        span { class: "home-continue-count", {(i18n_text.progress_count)(universe.done, universe.total)} }
                    }
                    span { class: "home-release-preview", {(i18n_text.universe_next)(&universe.next_title)} }
                }
            }
        }
    }
}

#[component]
fn ContinueCard(entry: ContinueEntry) -> Element {
    let i18n_text = use_texts();
    let ContinueEntry { item_id, category_id, subcategory_id, bar, on_hold, .. } = entry;

    let image_style = entry
        .image_url
        .as_ref()
        .map(|url| format!("background-image: url('{url}');"))
        .unwrap_or_default();

    rsx! {
        div { class: "home-release",
            Link {
                class: "home-release-link",
                to: Route::ItemDetail { id: item_id, category_id, subcategory_id },
                div { class: "home-release-image", style: "{image_style}" }
                div { class: "home-release-body",
                    span { class: "home-release-subcategory", "{entry.subcategory_name}" }
                    span { class: "home-release-title", "{entry.title}" }
                    if on_hold {
                        span { class: "home-status-chip", "{i18n_text.status_on_hold}" }
                    }
                    if bar.is_some() || entry.count.is_some() {
                        div { class: "home-continue-progress",
                            if let Some(percent) = bar {
                                div { class: "home-check-bar home-continue-bar",
                                    div { class: "home-check-bar-fill", width: "{percent}%" }
                                }
                            }
                            if let Some(count) = &entry.count {
                                span { class: "home-continue-count", "{count}" }
                            }
                        }
                    }
                    if let Some(next_line) = &entry.next_line {
                        span { class: "home-release-preview", "{next_line}" }
                    }
                }
            }
        }
    }
}
