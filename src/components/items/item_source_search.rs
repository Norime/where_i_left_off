use dioxus::prelude::*;
use serde_json::Value;

use crate::json_path;
use crate::models::{ApiSource, ItemInfoField};
use crate::net;
use crate::state::SourceStore;

#[derive(Clone, Debug, Default)]
pub struct FetchedItemData {
    pub title: String,
    pub image_url: Option<String>,
    pub description: String,
    pub extra_info: Vec<ItemInfoField>,
}

#[component]
pub fn ItemSourceSearch(scopes: Vec<(usize, usize)>, on_apply: EventHandler<FetchedItemData>) -> Element {
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
    let mut query = use_signal(String::new);
    let mut results = use_signal(Vec::<Value>::new);
    let mut is_loading = use_signal(|| false);
    let mut error_message = use_signal(|| Option::<String>::None);

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
                .and_then(|s| json_path::resolve_as_string(&result, &s.title_path))
                .unwrap_or_else(|| "Sans titre".to_string());
            (index, title, result)
        })
        .collect();

    if available_sources.is_empty() {
        return rsx! {
            p { class: "toolbar-hint",
                "Aucune source configurée pour cette sous-catégorie. Tu peux en ajouter une depuis le bouton 🔌 en haut de la page de la sous-catégorie."
            }
        };
    }

    rsx! {
        div { class: "source-search",
            div { class: "item-form-row",
                div { class: "item-form-field",
                    label { "Source" }
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
                    label { "Recherche" }
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
                                    Err(e) => error_message.set(Some(e.0)),
                                }
                                is_loading.set(false);
                            });
                        }
                    }
                },
                if *is_loading.read() {
                    "Recherche..."
                } else {
                    "Rechercher"
                }
            }

            if let Some(message) = error_message.read().clone() {
                p { class: "form-error", "{message}" }
            }

            if !result_rows.is_empty() {
                ul { class: "source-result-list",
                    for (index , title , result) in result_rows {
                        li { key: "{index}",
                            button {
                                r#type: "button",
                                class: "source-result-item",
                                onclick: {
                                    let source_for_click = selected_source.clone();
                                    move |_| {
                                        if let Some(source) = source_for_click.clone() {
                                            let data = extract_fetched_data(&source, &result);
                                            on_apply.call(data);
                                        }
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
    let title = json_path::resolve_as_string(result, &source.title_path).unwrap_or_default();

    let image_url = if source.image_path.trim().is_empty() {
        None
    } else {
        json_path::resolve_as_string(result, &source.image_path)
    };

    let description = if source.description_path.trim().is_empty() {
        String::new()
    } else {
        json_path::resolve_as_string(result, &source.description_path).unwrap_or_default()
    };

    let extra_info = source
        .extra_fields
        .iter()
        .filter_map(|field| {
            json_path::resolve_as_string(result, &field.json_path)
                .map(|value| ItemInfoField { label: field.label.clone(), value })
        })
        .collect();

    FetchedItemData { title, image_url, description, extra_info }
}