use dioxus::prelude::*;

use crate::components::sources::AddCatalogSourceModal;
use crate::components::LanguageSelect;
use crate::i18n::{use_texts, Language};
use crate::models::SourceScope;
use crate::source_catalog::{self, CatalogKind, CatalogSource};
use crate::state::{CategoryStore, SourceStore};

/// Catalogue de sources prêtes à l'emploi, groupées par type (lecture,
/// films, séries). Seules celles qui proposent du contenu dans la langue
/// choisie dans la liste déroulante sont affichées (par défaut celle de
/// l'interface, tant que l'utilisateur n'en a pas choisi une autre).
///
/// `fixed_scope` : affiché depuis la gestion des sources d'une
/// sous-catégorie, « Ajouter » vise directement celle-ci au lieu de
/// demander où ajouter la source.
#[component]
pub fn SourceCatalogPanel(#[props(default)] fixed_scope: Option<SourceScope>) -> Element {
    let i18n_text = use_texts();
    let language = use_context::<Memo<Language>>()();
    let source_store = use_context::<Signal<SourceStore>>();
    let category_store = use_context::<Signal<CategoryStore>>();
    let mut picked = use_signal(|| Option::<Language>::None);
    let content_language = picked().unwrap_or(language);
    let mut adding = use_signal(|| Option::<(&'static str, Language)>::None);

    let describe_scopes = |scopes: &[SourceScope]| -> String {
        let store = category_store.read();
        scopes
            .iter()
            .filter_map(|scope| {
                let category = store.categories.iter().find(|c| c.id == scope.category_id)?;
                let subcategory = category.subcategories.iter().find(|s| s.id == scope.subcategory_id)?;
                Some(format!("{} › {}", category.display_name(language), subcategory.display_name(language)))
            })
            .collect::<Vec<_>>()
            .join(", ")
    };

    let groups: Vec<(CatalogKind, Vec<&'static CatalogSource>)> = CatalogKind::ALL
        .iter()
        .map(|kind| {
            let entries = source_catalog::catalog()
                .iter()
                .filter(|entry| entry.kind == *kind && entry.supports(content_language))
                .collect();
            (*kind, entries)
        })
        .collect();

    rsx! {
        div { class: "catalog",
            div { class: "catalog-header",
                p { class: "toolbar-hint", {(i18n_text.catalog_intro)(content_language.native_name())} }
                div { class: "catalog-language",
                    span { "{i18n_text.catalog_language}" }
                    LanguageSelect {
                        value: content_language,
                        onchange: move |l| picked.set(Some(l)),
                    }
                }
            }

            for (kind , entries) in groups {
                section { key: "{kind:?}", class: "catalog-section",
                    h2 { class: "home-section-title",
                        match kind {
                            CatalogKind::Reading => i18n_text.catalog_reading,
                            CatalogKind::Movies => i18n_text.catalog_movies,
                            CatalogKind::Series => i18n_text.catalog_series,
                        }
                    }
                    if entries.is_empty() {
                        p { class: "toolbar-hint", "{i18n_text.catalog_empty_kind}" }
                    }
                    div { class: "catalog-grid",
                        for entry in entries {
                            {
                                let existing = source_store
                                    .read()
                                    .find_catalog_source(&entry.catalog_key(content_language))
                                    .cloned();
                                let linked_here = matches!(
                                    (&existing, fixed_scope),
                                    (Some(source), Some(scope)) if source.linked_scopes.contains(&scope)
                                );
                                let used_in = existing.as_ref().map(|s| describe_scopes(&s.linked_scopes));
                                rsx! {
                                    div { key: "{entry.id}", class: "catalog-card",
                                        div { class: "catalog-card-top",
                                            strong { class: "catalog-card-name", "{entry.name(content_language)}" }
                                        }
                                        span { class: "catalog-card-site", "{entry.website}" }
                                        p { class: "catalog-card-description", "{entry.description(language)}" }
                                        div { class: "catalog-badges",
                                            if entry.api_key_url.is_some() {
                                                span { class: "catalog-badge key", "🔑 {i18n_text.catalog_api_key_needed}" }
                                            }
                                            if !entry.build(content_language, "").list_levels.is_empty() {
                                                span { class: "catalog-badge list", "📋 {i18n_text.catalog_tracks_list}" }
                                            }
                                        }
                                        if let Some(used_in) = used_in.filter(|u| !u.is_empty()) {
                                            p { class: "catalog-card-used", {(i18n_text.catalog_added_in)(&used_in)} }
                                        }
                                        button {
                                            r#type: "button",
                                            class: "primary-btn catalog-add-btn",
                                            disabled: linked_here,
                                            onclick: {
                                                let id = entry.id;
                                                move |_| adding.set(Some((id, content_language)))
                                            },
                                            if linked_here {
                                                "{i18n_text.catalog_linked_here}"
                                            } else {
                                                "{i18n_text.catalog_add}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if let Some((entry_id, content_language)) = adding() {
                AddCatalogSourceModal {
                    entry_id,
                    content_language,
                    fixed_scope,
                    onclose: move |_| adding.set(None),
                }
            }
        }
    }
}
