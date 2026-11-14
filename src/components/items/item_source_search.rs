use dioxus::prelude::*;
use serde_json::Value;

use crate::i18n::use_texts;
use crate::models::{ApiSource, ImportedFrom, ItemInfoField, ProgressNode};
use crate::net;
use crate::source_import;
use crate::state::SourceStore;

#[derive(Clone, Debug, Default)]
pub struct FetchedItemData {
    pub title: String,
    pub image_url: Option<String>,
    pub description: String,
    pub extra_info: Vec<ItemInfoField>,
    /// Liste d'éléments importée (vide si la source n'en décrit pas, ou si
    /// `import_progress` est désactivé).
    pub progress: Vec<ProgressNode>,
    pub imported_from: Option<ImportedFrom>,
}

/// `import_progress` : au choix d'un résultat, importe aussi la liste
/// d'éléments décrite par la source (saisons, épisodes...) avant d'appeler
/// `on_apply`. `initial_query` pré-remplit le champ de recherche (ex: le
/// titre de l'œuvre quand on lui relie une source).
#[component]
pub fn ItemSourceSearch(
    scopes: Vec<(usize, usize)>,
    on_apply: EventHandler<FetchedItemData>,
    #[props(default)] import_progress: bool,
    #[props(default)] initial_query: String,
) -> Element {
    let i18n_text = use_texts();
    let source_store = use_context::<Signal<SourceStore>>();

    let available_sources: Vec<ApiSource> = source_store
    .read()
    .sources
    .iter()
    .filter(|s| {
        s.linked_scopes
            .iter()
            .any(|scope| scopes.contains(&(scope.category_id, scope.subcategory_id)))
    })
    .cloned()
    .collect();

    let mut selected_source_id = use_signal(|| available_sources.first().map(|s| s.id));
    let mut query = use_signal(|| initial_query.clone());
    let mut results = use_signal(Vec::<Value>::new);
    let mut is_loading = use_signal(|| false);
    let mut is_importing = use_signal(|| false);
    let mut error_message = use_signal(|| Option::<net::SearchError>::None);

    let selected_source: Option<ApiSource> = available_sources
        .iter()
        .find(|s| Some(s.id) == *selected_source_id.read())
        .cloned();

    let current_selected_id: Option<usize> = *selected_source_id.read();
    let selected_source_id_text = current_selected_id.map(|id| id.to_string()).unwrap_or_default();

    let result_rows: Vec<(usize, String, Value)> = results
        .read()
        .iter()
        .cloned()
        .enumerate()
        .map(|(index, result)| {
            let title = selected_source
                .as_ref()
                .and_then(|s| source_import::extract_text(&s.title_path, &result, &result))
                .unwrap_or_else(|| i18n_text.untitled.to_string());
            (index, title, result)
        })
        .collect();

    if available_sources.is_empty() {
        return rsx! {
            p { class: "toolbar-hint",
                "{i18n_text.no_source_configured}"
            }
        };
    }

    rsx! {
        div { class: "source-search",
            div { class: "item-form-row",
                div { class: "item-form-field",
                    label { "{i18n_text.source}" }
                    select {
                        value: "{selected_source_id_text}",
                        onchange: move |event| {
                            selected_source_id.set(event.value().parse().ok());
                        },
                        for source in available_sources.iter() {
                            option { value: "{source.id}", key: "{source.id}", "{source.name}" }
                        }
                    }
                }
                div { class: "item-form-field",
                    label { "{i18n_text.search_label}" }
                    input {
                        r#type: "text",
                        value: "{query}",
                        oninput: move |event| query.set(event.value()),
                    }
                }
            }
            button {
                r#type: "button",
                class: "secondary-btn",
                disabled: *is_loading.read(),
                onclick: move |_| {
                    if let Some(source) = selected_source.clone() {
                        let search_query = query.read().clone();
                        if !search_query.trim().is_empty() {
                            is_loading.set(true);
                            error_message.set(None);
                            spawn(async move {
                                match net::search(&source, &search_query).await {
                                    Ok(found) => results.set(found),
                                    Err(e) => error_message.set(Some(e)),
                                }
                                is_loading.set(false);
                            });
                        }
                    }
                },
                if *is_loading.read() {
                    "{i18n_text.searching}"
                } else {
                    "{i18n_text.search_button}"
                }
            }

            if let Some(error) = error_message.read().clone() {
                p { class: "form-error", {i18n_text.search_error(&error)} }
            }

            if *is_importing.read() {
                p { class: "toolbar-hint", "{i18n_text.importing_list}" }
            }

            if !result_rows.is_empty() {
                ul { class: "source-result-list",
                    for (index , title , result) in result_rows {
                        li { key: "{index}",
                            button {
                                r#type: "button",
                                class: "source-result-item",
                                disabled: *is_importing.read(),
                                onclick: {
                                    let source_for_click = selected_source.clone();
                                    move |_| {
                                        let Some(source) = source_for_click.clone() else {
                                            return;
                                        };
                                        let mut data = extract_fetched_data(&source, &result);
                                        if !import_progress || source.list_levels.is_empty() {
                                            on_apply.call(data);
                                            return;
                                        }

                                        let root_result = result.clone();
                                        is_importing.set(true);
                                        error_message.set(None);
                                        spawn(async move {
                                            match source_import::fetch_progress(
                                                    &source,
                                                    &root_result,
                                                    source_import::MAX_PAGES,
                                                )
                                                .await
                                            {
                                                Ok(progress) => data.progress = progress,
                                                Err(e) => error_message.set(Some(e)),
                                            }
                                            // Gardé même en cas d'erreur : la liste pourra être
                                            // réimportée plus tard depuis la fiche.
                                            data.imported_from = Some(ImportedFrom {
                                                source_id: source.id,
                                                root_result,
                                            });
                                            is_importing.set(false);
                                            on_apply.call(data);
                                        });
                                    }
                                },
                                "{title}"
                            }
                        }
                    }
                }
            }
        }
    }
}

fn extract_fetched_data(source: &ApiSource, result: &Value) -> FetchedItemData {
    // Chemins JSON ou gabarits (ex: "https://image.tmdb.org/t/p/w300{poster_path}").
    let extract = |path: &str| source_import::extract_text(path, result, result);
    let title = extract(&source.title_path).unwrap_or_default();
    let image_url = extract(&source.image_path);
    let description = extract(&source.description_path).unwrap_or_default();

    let extra_info = source
        .extra_fields
        .iter()
        .filter_map(|field| {
            extract(&field.json_path).map(|value| ItemInfoField { label: field.label.clone(), value })
        })
        .collect();

    FetchedItemData { title, image_url, description, extra_info, ..Default::default() }
}