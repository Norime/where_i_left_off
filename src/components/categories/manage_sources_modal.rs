use dioxus::prelude::*;

use crate::components::{DeleteButton, Modal};
use crate::json_path;
use crate::models::{ApiSource, SourceField, SourceScope};
use crate::net;
use crate::state::{CategoryStore, SourceStore};

#[component]
pub fn ManageSourcesModal(category_id: usize, subcategory_id: usize, oncancel: EventHandler<()>) -> Element {
    let mut source_store = use_context::<Signal<SourceStore>>();
    let category_store = use_context::<Signal<CategoryStore>>();

    let current_scope = SourceScope { category_id, subcategory_id };

    let all_sources: Vec<ApiSource> = source_store.read().sources.clone();

    let linked_sources: Vec<ApiSource> = all_sources
        .iter()
        .filter(|s| s.linked_scopes.contains(&current_scope))
        .cloned()
        .collect();

    let same_category_candidates: Vec<ApiSource> = all_sources
        .iter()
        .filter(|s| !s.linked_scopes.contains(&current_scope))
        .filter(|s| s.linked_scopes.iter().any(|scope| scope.category_id == category_id))
        .cloned()
        .collect();

    let other_category_candidates: Vec<ApiSource> = all_sources
        .iter()
        .filter(|s| !s.linked_scopes.contains(&current_scope))
        .filter(|s| s.linked_scopes.iter().all(|scope| scope.category_id != category_id))
        .cloned()
        .collect();

    let describe = |scopes: &[SourceScope]| -> String {
        let store = category_store.read();
        scopes
            .iter()
            .filter_map(|scope| {
                let category = store.categories.iter().find(|c| c.id == scope.category_id)?;
                let subcategory = category.subcategories.iter().find(|s| s.id == scope.subcategory_id)?;
                Some(format!("{} > {}", category.name, subcategory.name))
            })
            .collect::<Vec<_>>()
            .join(", ")
    };

    let mut editing = use_signal(|| Option::<ApiSource>::None);
    let editing_draft = editing.read().clone();

    rsx! {
        Modal { onclose: move |_| oncancel.call(()),
            div { class: "manage-sources",
                h2 { "Sources de données pour cette sous-catégorie" }
                p { class: "toolbar-hint",
                    "Une source interroge une API en lecture seule (GET, sans authentification) avec un titre, et sait en extraire un titre, une image, une description et des champs additionnels grâce à des chemins JSON que tu définis toi-même."
                }

                h3 { "Reliées ici" }
                if linked_sources.is_empty() {
                    p { class: "toolbar-hint",
                        "Aucune source reliée à cette sous-catégorie pour l'instant."
                    }
                }
                ul { class: "source-list",
                    for source in linked_sources.iter().cloned() {
                        li { key: "{source.id}", class: "source-list-item",
                            span { "{source.name}" }
                            div { class: "source-list-actions",
                                button {
                                    r#type: "button",
                                    class: "secondary-btn",
                                    onclick: {
                                        let source = source.clone();
                                        move |_| editing.set(Some(source.clone()))
                                    },
                                    "Modifier"
                                }
                                DeleteButton {
                                    onconfirm: move |_| {
                                        source_store.write().unlink_source(source.id, category_id, subcategory_id);
                                    },
                                }
                            }
                        }
                    }
                }

                button {
                    r#type: "button",
                    class: "primary-btn",
                    onclick: move |_| {
                        editing
                            .set(
                                Some(ApiSource {
                                    id: 0,
                                    linked_scopes: vec![current_scope],
                                    name: String::new(),
                                    search_url_template: String::new(),
                                    results_path: String::new(),
                                    title_path: String::new(),
                                    image_path: String::new(),
                                    description_path: String::new(),
                                    extra_fields: Vec::new(),
                                }),
                            );
                    },
                    "+ Nouvelle source"
                }

                div { class: "manage-sources-columns",
                    div { class: "manage-sources-column",
                        h3 { "Dans ma catégorie" }
                        if same_category_candidates.is_empty() {
                            p { class: "toolbar-hint", "Aucune autre source dans cette catégorie." }
                        }
                        ul { class: "source-list",
                            for source in same_category_candidates.iter().cloned() {
                                li {
                                    key: "{source.id}",
                                    class: "source-list-item",
                                    div {
                                        span { "{source.name}" }
                                        p { class: "source-usage-hint",
                                            "Utilisée dans : {describe(&source.linked_scopes)}"
                                        }
                                    }
                                    button {
                                        r#type: "button",
                                        class: "secondary-btn",
                                        onclick: {
                                            let source_id = source.id;
                                            move |_| {
                                                source_store.write().link_source(source_id, category_id, subcategory_id);
                                            }
                                        },
                                        "Lier"
                                    }
                                }
                            }
                        }
                    }
                    div { class: "manage-sources-column",
                        h3 { "Autres catégories" }
                        if other_category_candidates.is_empty() {
                            p { class: "toolbar-hint", "Aucune source dans une autre catégorie." }
                        }
                        ul { class: "source-list",
                            for source in other_category_candidates.iter().cloned() {
                                li {
                                    key: "{source.id}",
                                    class: "source-list-item",
                                    div {
                                        span { "{source.name}" }
                                        p { class: "source-usage-hint",
                                            "Utilisée dans : {describe(&source.linked_scopes)}"
                                        }
                                    }
                                    button {
                                        r#type: "button",
                                        class: "secondary-btn",
                                        onclick: {
                                            let source_id = source.id;
                                            move |_| {
                                                source_store.write().link_source(source_id, category_id, subcategory_id);
                                            }
                                        },
                                        "Lier"
                                    }
                                }
                            }
                        }
                    }
                }

                if let Some(draft) = editing_draft {
                    SourceEditor {
                        draft,
                        oncancel: move |_| editing.set(None),
                        onsave: move |saved: ApiSource| {
                            if saved.id == 0 {
                                source_store.write().add_source(saved);
                            } else {
                                source_store.write().update_source(saved);
                            }
                            editing.set(None);
                        },
                    }
                }
            }
        }
    }
}

#[component]
fn SourceEditor(draft: ApiSource, oncancel: EventHandler<()>, onsave: EventHandler<ApiSource>) -> Element {
    let mut name = use_signal(|| draft.name.clone());
    let mut search_url_template = use_signal(|| draft.search_url_template.clone());
    let mut results_path = use_signal(|| draft.results_path.clone());
    let mut title_path = use_signal(|| draft.title_path.clone());
    let mut image_path = use_signal(|| draft.image_path.clone());
    let mut description_path = use_signal(|| draft.description_path.clone());
    let mut extra_fields = use_signal(|| draft.extra_fields.clone());

    let mut new_field_label = use_signal(String::new);
    let mut new_field_path = use_signal(String::new);

    let mut test_query = use_signal(String::new);
    let mut test_raw_result = use_signal(|| Option::<serde_json::Value>::None);
    let mut test_error = use_signal(|| Option::<String>::None);
    let mut is_testing = use_signal(|| false);

    let draft_id = draft.id;
    let draft_linked_scopes = draft.linked_scopes.clone();
    let is_shared = draft.linked_scopes.len() > 1;

    let extra_field_rows: Vec<(usize, String, String)> = extra_fields
        .read()
        .iter()
        .cloned()
        .enumerate()
        .map(|(index, field)| (index, field.label, field.json_path))
        .collect();

    let raw_result = test_raw_result.read().clone();
    let preview_title_text = raw_result
        .as_ref()
        .and_then(|v| json_path::resolve_as_string(v, &title_path.read()))
        .unwrap_or_else(|| "(rien)".to_string());
    let preview_image_text = raw_result
        .as_ref()
        .and_then(|v| json_path::resolve_as_string(v, &image_path.read()))
        .unwrap_or_else(|| "(rien)".to_string());
    let preview_description_text = raw_result
        .as_ref()
        .and_then(|v| json_path::resolve_as_string(v, &description_path.read()))
        .unwrap_or_else(|| "(rien)".to_string());
    let has_test_result = raw_result.is_some();

    rsx! {
        div { class: "source-editor",
            if is_shared {
                p { class: "form-error",
                    "Cette source est reliée à plusieurs sous-catégories : la modifier changera son comportement partout où elle est utilisée."
                }
            }
            div { class: "item-form-field",
                label { "Nom de la source" }
                input {
                    r#type: "text",
                    value: "{name}",
                    oninput: move |e| name.set(e.value()),
                }
            }
            div { class: "item-form-field",
                label { "URL de recherche (avec {{query}})" }
                input {
                    r#type: "text",
                    placeholder: "https://api.jikan.moe/v4/anime?q={{query}}",
                    value: "{search_url_template}",
                    oninput: move |e| search_url_template.set(e.value()),
                }
            }
            div { class: "item-form-row",
                div { class: "item-form-field",
                    label { "Chemin vers la liste de résultats" }
                    input {
                        r#type: "text",
                        placeholder: "data",
                        value: "{results_path}",
                        oninput: move |e| results_path.set(e.value()),
                    }
                }
                div { class: "item-form-field",
                    label { "Chemin vers le titre" }
                    input {
                        r#type: "text",
                        placeholder: "title",
                        value: "{title_path}",
                        oninput: move |e| title_path.set(e.value()),
                    }
                }
            }
            div { class: "item-form-row",
                div { class: "item-form-field",
                    label { "Chemin vers l'image" }
                    input {
                        r#type: "text",
                        placeholder: "images.jpg.image_url",
                        value: "{image_path}",
                        oninput: move |e| image_path.set(e.value()),
                    }
                }
                div { class: "item-form-field",
                    label { "Chemin vers la description" }
                    input {
                        r#type: "text",
                        placeholder: "synopsis",
                        value: "{description_path}",
                        oninput: move |e| description_path.set(e.value()),
                    }
                }
            }

            h3 { "Champs additionnels (tomes, épisodes, saisons...)" }
            ul { class: "source-field-list",
                for (index , label , path) in extra_field_rows {
                    li { key: "{index}",
                        span { "{label} → {path}" }
                        button {
                            r#type: "button",
                            class: "danger-btn",
                            onclick: move |_| {
                                extra_fields.write().remove(index);
                            },
                            "×"
                        }
                    }
                }
            }
            div { class: "item-form-row",
                input {
                    r#type: "text",
                    placeholder: "Label (ex: Tomes)",
                    value: "{new_field_label}",
                    oninput: move |e| new_field_label.set(e.value()),
                }
                input {
                    r#type: "text",
                    placeholder: "Chemin JSON (ex: volumes)",
                    value: "{new_field_path}",
                    oninput: move |e| new_field_path.set(e.value()),
                }
                button {
                    r#type: "button",
                    class: "secondary-btn",
                    onclick: move |_| {
                        let label = new_field_label.read().trim().to_string();
                        let path = new_field_path.read().trim().to_string();
                        if !label.is_empty() && !path.is_empty() {
                            extra_fields
                                .write()
                                .push(SourceField {
                                    label,
                                    json_path: path,
                                });
                            new_field_label.set(String::new());
                            new_field_path.set(String::new());
                        }
                    },
                    "Ajouter le champ"
                }
            }

            h3 { "Tester" }
            p { class: "toolbar-hint",
                "Fais une recherche d'essai pour vérifier que tes chemins renvoient bien les bonnes valeurs."
            }
            div { class: "item-form-row",
                input {
                    r#type: "text",
                    placeholder: "Ex: Mushoku Tensei",
                    value: "{test_query}",
                    oninput: move |e| test_query.set(e.value()),
                }
                button {
                    r#type: "button",
                    class: "secondary-btn",
                    disabled: *is_testing.read(),
                    onclick: {
                        let linked_scopes_for_test = draft_linked_scopes.clone();
                        move |_| {
                            let candidate = ApiSource {
                                id: draft_id,
                                linked_scopes: linked_scopes_for_test.clone(),
                                name: name.read().clone(),
                                search_url_template: search_url_template.read().clone(),
                                results_path: results_path.read().clone(),
                                title_path: title_path.read().clone(),
                                image_path: image_path.read().clone(),
                                description_path: description_path.read().clone(),
                                extra_fields: extra_fields.read().clone(),
                            };
                            let query = test_query.read().clone();
                            if query.trim().is_empty() {
                                return;
                            }
                            is_testing.set(true);
                            test_error.set(None);
                            spawn(async move {
                                match net::search(&candidate, &query).await {
                                    Ok(found) => test_raw_result.set(found.into_iter().next()),
                                    Err(e) => test_error.set(Some(e.0)),
                                }
                                is_testing.set(false);
                            });
                        }
                    },
                    if *is_testing.read() {
                        "Test en cours..."
                    } else {
                        "Lancer le test"
                    }
                }
            }

            if let Some(message) = test_error.read().clone() {
                p { class: "form-error", "{message}" }
            }

            if has_test_result {
                div { class: "source-test-preview",
                    p {
                        "Titre extrait : "
                        strong { "{preview_title_text}" }
                    }
                    p {
                        "Image extraite : "
                        strong { "{preview_image_text}" }
                    }
                    p {
                        "Description extraite : "
                        strong { "{preview_description_text}" }
                    }
                }
            }

            div { class: "modal-actions",
                button {
                    r#type: "button",
                    class: "secondary-btn",
                    onclick: move |_| oncancel.call(()),
                    "Annuler"
                }
                button {
                    r#type: "button",
                    class: "primary-btn",
                    onclick: {
                        let linked_scopes_for_save = draft_linked_scopes.clone();
                        move |_| {
                            onsave
                                .call(ApiSource {
                                    id: draft_id,
                                    linked_scopes: linked_scopes_for_save.clone(),
                                    name: name.read().clone(),
                                    search_url_template: search_url_template.read().clone(),
                                    results_path: results_path.read().clone(),
                                    title_path: title_path.read().clone(),
                                    image_path: image_path.read().clone(),
                                    description_path: description_path.read().clone(),
                                    extra_fields: extra_fields.read().clone(),
                                });
                        }
                    },
                    "Enregistrer"
                }
            }
        }
    }
}