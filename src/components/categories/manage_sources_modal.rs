use std::collections::HashMap;

use dioxus::prelude::*;
use serde_json::Value;

use crate::components::sources::SourceCatalogPanel;
use crate::components::{DeleteButton, Modal};
use crate::i18n::use_texts;
use crate::models::{ApiSource, ListPagination, ProgressNode, ProgressNodeKind, SourceField, SourceListLevel, SourceScope};
use crate::net;
use crate::source_import;
use crate::state::{CategoryStore, SourceStore};

/// Ce que montre la fenêtre de gestion des sources.
#[derive(Clone, PartialEq)]
enum ManageView {
    /// Sources reliées + choix pour en ajouter une.
    List,
    /// Catalogue de sources prêtes à l'emploi.
    Catalog,
    /// Création (`is_new`) ou modification d'une source.
    Edit { source: ApiSource, is_new: bool },
}

#[component]
pub fn ManageSourcesModal(category_id: usize, subcategory_id: usize, oncancel: EventHandler<()>) -> Element {
    let i18n_text = use_texts();
    let mut source_store = use_context::<Signal<SourceStore>>();
    let category_store = use_context::<Signal<CategoryStore>>();
    let mut view = use_signal(|| ManageView::List);

    let current_scope = SourceScope { category_id, subcategory_id };
    let all_sources: Vec<ApiSource> = source_store.read().sources.clone();
    let linked_sources: Vec<ApiSource> =
        all_sources.iter().filter(|s| s.linked_scopes.contains(&current_scope)).cloned().collect();
    // Sources réutilisables : celles de la même catégorie d'abord.
    let mut reusable: Vec<ApiSource> =
        all_sources.iter().filter(|s| !s.linked_scopes.contains(&current_scope)).cloned().collect();
    reusable.sort_by_key(|s| !s.linked_scopes.iter().any(|scope| scope.category_id == category_id));

    let describe = |scopes: &[SourceScope]| -> String {
        let store = category_store.read();
        scopes
            .iter()
            .filter_map(|scope| {
                let category = store.categories.iter().find(|c| c.id == scope.category_id)?;
                let subcategory = category.subcategories.iter().find(|s| s.id == scope.subcategory_id)?;
                Some(format!("{} › {}", category.name, subcategory.name))
            })
            .collect::<Vec<_>>()
            .join(", ")
    };
    let here = describe(&[current_scope]);

    let content = match view() {
        ManageView::Edit { source: draft, is_new } => rsx! {
            SourceEditor {
                draft,
                oncancel: move |_| view.set(ManageView::List),
                onsave: move |saved: ApiSource| {
                    if is_new {
                        source_store.write().add_source(saved);
                    } else {
                        source_store.write().update_source(saved);
                    }
                    view.set(ManageView::List);
                },
            }
        },
        ManageView::Catalog => rsx! {
            button {
                r#type: "button",
                class: "link-btn",
                onclick: move |_| view.set(ManageView::List),
                "{i18n_text.back_to_sources}"
            }
            SourceCatalogPanel { fixed_scope: Some(current_scope) }
        },
        ManageView::List => rsx! {
            p { class: "toolbar-hint", "{i18n_text.sources_intro}" }

            h3 { "{i18n_text.linked_here}" }
            if linked_sources.is_empty() {
                p { class: "source-empty", "{i18n_text.no_linked_source}" }
            }
            div { class: "source-card-list",
                for source in linked_sources.iter().cloned() {
                    div { key: "{source.id}", class: "source-card",
                        div { class: "source-card-main",
                            strong { "{source.name}" }
                            span { class: "source-card-meta",
                                "{source_host(&source)} · "
                                {levels_summary(&source).unwrap_or_else(|| i18n_text.info_only.to_string())}
                            }
                            if source.linked_scopes.len() > 1 {
                                span { class: "source-card-meta", {(i18n_text.used_in)(&describe(&source.linked_scopes))} }
                            }
                        }
                        div { class: "source-list-actions",
                            button {
                                r#type: "button",
                                class: "secondary-btn",
                                onclick: {
                                    let source = source.clone();
                                    move |_| view.set(ManageView::Edit { source: source.clone(), is_new: false })
                                },
                                "{i18n_text.edit}"
                            }
                            DeleteButton {
                                onconfirm: move |_| source_store.write().unlink_source(source.id, category_id, subcategory_id),
                            }
                        }
                    }
                }
            }

            h3 { "{i18n_text.add_source_title}" }
            div { class: "source-add-choices",
                button {
                    r#type: "button",
                    class: "source-add-choice",
                    onclick: move |_| view.set(ManageView::Catalog),
                    strong { "{i18n_text.from_catalog}" }
                    span { "{i18n_text.from_catalog_hint}" }
                }
                button {
                    r#type: "button",
                    class: "source-add-choice",
                    onclick: move |_| {
                        view.set(ManageView::Edit {
                            source: ApiSource { linked_scopes: vec![current_scope], ..Default::default() },
                            is_new: true,
                        })
                    },
                    strong { "{i18n_text.custom_source}" }
                    span { "{i18n_text.custom_source_hint}" }
                }
            }

            if !reusable.is_empty() {
                details { class: "source-reuse",
                    summary { {(i18n_text.reuse_existing)(reusable.len())} }
                    div { class: "source-card-list",
                        for source in reusable.iter().cloned() {
                            div { key: "{source.id}", class: "source-card",
                                div { class: "source-card-main",
                                    strong { "{source.name}" }
                                    span { class: "source-card-meta", {(i18n_text.used_in)(&describe(&source.linked_scopes))} }
                                }
                                button {
                                    r#type: "button",
                                    class: "secondary-btn",
                                    onclick: move |_| source_store.write().link_source(source.id, category_id, subcategory_id),
                                    "{i18n_text.link}"
                                }
                            }
                        }
                    }
                }
            }

            div { class: "modal-actions",
                button {
                    r#type: "button",
                    class: "secondary-btn",
                    onclick: move |_| oncancel.call(()),
                    "{i18n_text.close}"
                }
            }
        },
    };

    rsx! {
        Modal { class: "modal-wide", onclose: move |_| oncancel.call(()),
            div { class: "manage-sources",
                h2 { "{i18n_text.sources_for_subcategory}" }
                p { class: "manage-sources-scope", "{here}" }
                {content}
            }
        }
    }
}

/// Domaine de l'API (ex: "api.themoviedb.org"), pour reconnaître la source.
fn source_host(source: &ApiSource) -> String {
    let url = source.search_url_template.trim();
    let without_scheme = url.split_once("://").map(|(_, rest)| rest).unwrap_or(url);
    without_scheme.split(['/', '?']).next().unwrap_or_default().to_string()
}

/// "Saisons → Épisodes", ou `None` si la source ne remplit que la fiche.
fn levels_summary(source: &ApiSource) -> Option<String> {
    (!source.list_levels.is_empty())
        .then(|| source.list_levels.iter().map(|l| l.label.clone()).collect::<Vec<_>>().join(" → "))
}

#[component]
fn SourceEditor(draft: ApiSource, oncancel: EventHandler<()>, onsave: EventHandler<ApiSource>) -> Element {
    let i18n_text = use_texts();
    let mut name = use_signal(|| draft.name.clone());
    let mut search_url_template = use_signal(|| draft.search_url_template.clone());
    let mut results_path = use_signal(|| draft.results_path.clone());
    let mut title_path = use_signal(|| draft.title_path.clone());
    let mut image_path = use_signal(|| draft.image_path.clone());
    let mut description_path = use_signal(|| draft.description_path.clone());
    let mut extra_fields = use_signal(|| draft.extra_fields.clone());
    let list_levels = use_signal(|| draft.list_levels.clone());

    let mut new_field_label = use_signal(String::new);
    let mut new_field_path = use_signal(String::new);

    let mut test_query = use_signal(String::new);
    let mut test_body = use_signal(|| Option::<Value>::None);
    let mut test_error = use_signal(|| Option::<net::SearchError>::None);
    let mut is_testing = use_signal(|| false);
    let mut test_progress = use_signal(|| Option::<Vec<ProgressNode>>::None);
    let mut test_list_error = use_signal(|| Option::<net::SearchError>::None);

    let is_shared = draft.linked_scopes.len() > 1;
    let build_source = {
        let draft = draft.clone();
        move || ApiSource {
            name: name.read().trim().to_string(),
            search_url_template: search_url_template.read().trim().to_string(),
            results_path: results_path.read().clone(),
            title_path: title_path.read().clone(),
            image_path: image_path.read().clone(),
            description_path: description_path.read().clone(),
            extra_fields: extra_fields.read().clone(),
            list_levels: list_levels.read().clone(),
            ..draft.clone()
        }
    };

    // Les résultats sont relus dans la réponse à chaque rendu : modifier le
    // chemin des résultats ou d'un champ met l'aperçu à jour aussitôt.
    let results: Vec<Value> = test_body.read().as_ref().map(|body| net::results_array(body, &results_path.read())).unwrap_or_default();
    let first = results.first().cloned();
    let extract = |path: &str| first.as_ref().and_then(|f| source_import::extract_text(path, f, f));
    let preview_title = extract(&title_path.read());
    let preview_image = extract(&image_path.read());
    let preview_description = extract(&description_path.read());
    let array_suggestions: Vec<String> = match (test_body.read().as_ref(), results.is_empty()) {
        (Some(body), true) => array_paths(body),
        _ => Vec::new(),
    };
    let available_fields: Vec<(String, String)> = first.as_ref().map(json_leaves).unwrap_or_default();

    let extra_field_rows: Vec<(usize, SourceField, Option<String>)> = extra_fields
        .read()
        .iter()
        .cloned()
        .enumerate()
        .map(|(index, field)| {
            let value = extract(&field.json_path);
            (index, field, value)
        })
        .collect();
    let level_count = list_levels.read().len();
    let list_preview: Option<Vec<(String, usize, String)>> =
        test_progress.read().as_ref().map(|nodes| list_preview_rows(&list_levels.read(), nodes));
    let missing_query = !search_url_template.read().trim().is_empty() && !search_url_template.read().contains("{query}");
    let can_save = !name.read().trim().is_empty() && !search_url_template.read().trim().is_empty();

    let run_test = {
        let build_source = build_source.clone();
        move |_| {
            let candidate = build_source();
            let query = test_query.read().trim().to_string();
            if query.is_empty() || candidate.search_url_template.is_empty() {
                return;
            }
            is_testing.set(true);
            test_error.set(None);
            test_list_error.set(None);
            test_progress.set(None);
            spawn(async move {
                match net::search_raw(&candidate, &query).await {
                    Ok(body) => {
                        let first = net::results_array(&body, &candidate.results_path).into_iter().next();
                        test_body.set(Some(body));
                        // Une seule page par liste : on vérifie les chemins,
                        // pas besoin de tout télécharger.
                        if let Some(first) = first.filter(|_| !candidate.list_levels.is_empty()) {
                            match source_import::fetch_progress(&candidate, &first, 1).await {
                                Ok(nodes) => test_progress.set(Some(nodes)),
                                Err(e) => test_list_error.set(Some(e)),
                            }
                        }
                    }
                    Err(e) => {
                        test_body.set(None);
                        test_error.set(Some(e));
                    }
                }
                is_testing.set(false);
            });
        }
    };

    rsx! {
        div { class: "source-editor",
            button { r#type: "button", class: "link-btn", onclick: move |_| oncancel.call(()), "{i18n_text.back_to_sources}" }
            if is_shared {
                p { class: "form-error", "{i18n_text.shared_source_warning}" }
            }

            // 1. Où chercher
            section { class: "editor-step",
                h3 { span { class: "editor-step-number", "1" } "{i18n_text.step_where}" }
                div { class: "item-form-field",
                    label { "{i18n_text.source_name}" }
                    input {
                        r#type: "text",
                        placeholder: "TMDB, MangaDex…",
                        value: "{name}",
                        oninput: move |e| name.set(e.value()),
                    }
                }
                div { class: "item-form-field",
                    label { "{i18n_text.search_url_label}" }
                    input {
                        r#type: "text",
                        class: "mono",
                        placeholder: "https://api.jikan.moe/v4/anime?q={{query}}",
                        value: "{search_url_template}",
                        oninput: move |e| search_url_template.set(e.value()),
                    }
                    span { class: "field-hint", "{i18n_text.search_url_hint}" }
                    if missing_query {
                        span { class: "form-error", "{i18n_text.missing_query_marker}" }
                    }
                }
            }

            // 2. Essayer
            section { class: "editor-step",
                h3 { span { class: "editor-step-number", "2" } "{i18n_text.step_try}" }
                div { class: "editor-test-row",
                    input {
                        r#type: "text",
                        placeholder: "{i18n_text.test_query_placeholder}",
                        value: "{test_query}",
                        oninput: move |e| test_query.set(e.value()),
                        onkeydown: {
                            let mut run_test = run_test.clone();
                            move |e: KeyboardEvent| {
                                if e.key() == Key::Enter {
                                    run_test(());
                                }
                            }
                        },
                    }
                    button {
                        r#type: "button",
                        class: "primary-btn",
                        disabled: is_testing() || search_url_template.read().trim().is_empty(),
                        onclick: {
                            let mut run_test = run_test.clone();
                            move |_| run_test(())
                        },
                        if is_testing() { "{i18n_text.testing}" } else { "{i18n_text.run_test}" }
                    }
                }
                if let Some(error) = test_error.read().clone() {
                    p { class: "form-error", {i18n_text.search_error(&error)} }
                }
                if test_body.read().is_some() {
                    if results.is_empty() {
                        p { class: "form-error", "{i18n_text.test_no_results}" }
                        div { class: "editor-suggestions",
                            for path in array_suggestions {
                                button {
                                    key: "{path}",
                                    r#type: "button",
                                    class: "chip-btn mono",
                                    onclick: {
                                        let path = path.clone();
                                        move |_| results_path.set(path.clone())
                                    },
                                    if path.is_empty() { "{i18n_text.root_path}" } else { "{path}" }
                                }
                            }
                        }
                    } else {
                        p { class: "editor-success", {(i18n_text.test_found)(results.len())} }
                    }
                }
            }

            // 3. Que garder
            section { class: "editor-step",
                h3 { span { class: "editor-step-number", "3" } "{i18n_text.step_extract}" }
                span { class: "field-hint", "{i18n_text.path_hint}" }
                div { class: "editor-extract",
                    div { class: "editor-extract-fields",
                        PathInput {
                            label: i18n_text.results_path_label,
                            placeholder: "data",
                            value: results_path,
                            preview: test_body.read().as_ref().map(|_| (i18n_text.test_found)(results.len())),
                        }
                        PathInput { label: i18n_text.title_path_label, placeholder: "title", value: title_path, preview: preview_title.clone() }
                        PathInput { label: i18n_text.image_path_label, placeholder: "images.jpg.image_url", value: image_path, preview: preview_image.clone() }
                        PathInput {
                            label: i18n_text.description_path_label,
                            placeholder: "synopsis",
                            value: description_path,
                            preview: preview_description.clone(),
                        }
                    }
                    div { class: "editor-preview",
                        span { class: "editor-preview-label", "{i18n_text.preview_label}" }
                        if first.is_some() {
                            div { class: "editor-preview-card",
                                div {
                                    class: "editor-preview-image",
                                    style: preview_image.as_ref().map(|url| format!("background-image: url('{url}');")).unwrap_or_default(),
                                }
                                div { class: "editor-preview-body",
                                    strong { {preview_title.clone().unwrap_or_else(|| i18n_text.untitled.to_string())} }
                                    p { {preview_description.clone().unwrap_or_default()} }
                                }
                            }
                        } else {
                            p { class: "toolbar-hint", "{i18n_text.run_test_first}" }
                        }
                    }
                }
                if !available_fields.is_empty() {
                    details { class: "editor-explorer",
                        summary { "{i18n_text.available_fields}" }
                        p { class: "field-hint", "{i18n_text.available_fields_hint}" }
                        div { class: "editor-explorer-rows",
                            for (path , value) in available_fields {
                                div { key: "{path}", class: "editor-explorer-row",
                                    code { "{path}" }
                                    span { class: "editor-explorer-value", "{value}" }
                                    div { class: "editor-explorer-actions",
                                        button {
                                            r#type: "button",
                                            class: "chip-btn",
                                            onclick: {
                                                let path = path.clone();
                                                move |_| title_path.set(path.clone())
                                            },
                                            "{i18n_text.use_as_title}"
                                        }
                                        button {
                                            r#type: "button",
                                            class: "chip-btn",
                                            onclick: {
                                                let path = path.clone();
                                                move |_| image_path.set(path.clone())
                                            },
                                            "{i18n_text.use_as_image}"
                                        }
                                        button {
                                            r#type: "button",
                                            class: "chip-btn",
                                            onclick: {
                                                let path = path.clone();
                                                move |_| description_path.set(path.clone())
                                            },
                                            "{i18n_text.use_as_description}"
                                        }
                                        button {
                                            r#type: "button",
                                            class: "chip-btn",
                                            onclick: {
                                                let path = path.clone();
                                                move |_| new_field_path.set(path.clone())
                                            },
                                            "{i18n_text.use_as_info}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 4. Informations supplémentaires
            section { class: "editor-step",
                h3 { span { class: "editor-step-number", "4" } "{i18n_text.extra_fields_title}" }
                div { class: "editor-chips",
                    for (index , field , value) in extra_field_rows {
                        span { key: "{index}", class: "editor-chip",
                            strong { "{field.label}" }
                            code { "{field.json_path}" }
                            if let Some(value) = value {
                                span { class: "editor-chip-value", "= {value}" }
                            }
                            button {
                                r#type: "button",
                                class: "editor-chip-remove",
                                onclick: move |_| {
                                    extra_fields.write().remove(index);
                                },
                                "×"
                            }
                        }
                    }
                }
                div { class: "editor-add-row",
                    input {
                        r#type: "text",
                        placeholder: "{i18n_text.extra_field_label_placeholder}",
                        value: "{new_field_label}",
                        oninput: move |e| new_field_label.set(e.value()),
                    }
                    input {
                        r#type: "text",
                        class: "mono",
                        placeholder: "{i18n_text.json_path_placeholder}",
                        value: "{new_field_path}",
                        oninput: move |e| new_field_path.set(e.value()),
                    }
                    button {
                        r#type: "button",
                        class: "secondary-btn",
                        onclick: move |_| {
                            let label = new_field_label.read().trim().to_string();
                            let json_path = new_field_path.read().trim().to_string();
                            if !label.is_empty() && !json_path.is_empty() {
                                extra_fields.write().push(SourceField { label, json_path });
                                new_field_label.set(String::new());
                                new_field_path.set(String::new());
                            }
                        },
                        "{i18n_text.add_field}"
                    }
                }
            }

            // 5. Liste à suivre
            section { class: "editor-step",
                h3 { span { class: "editor-step-number", "5" } "{i18n_text.step_list}" }
                span { class: "field-hint", "{i18n_text.list_levels_hint}" }
                for index in 0..level_count {
                    ListLevelEditor { key: "{index}", levels: list_levels, index }
                }
                button {
                    r#type: "button",
                    class: "secondary-btn editor-add-level",
                    onclick: move |_| {
                        let mut levels = list_levels;
                        levels.write().push(SourceListLevel { leaf_kind: ProgressNodeKind::Card, ..Default::default() });
                    },
                    "{i18n_text.add_list_level}"
                }
                if let Some(rows) = list_preview {
                    div { class: "editor-list-preview",
                        span { class: "editor-preview-label", "{i18n_text.preview_label}" }
                        for (index , (label , count , first_titles)) in rows.into_iter().enumerate() {
                            p { key: "{index}",
                                strong { {(i18n_text.list_level_preview)(&label, count)} }
                                " {first_titles}"
                            }
                        }
                    }
                }
                if let Some(error) = test_list_error.read().clone() {
                    p { class: "form-error", {i18n_text.search_error(&error)} }
                }
            }

            div { class: "modal-actions editor-footer",
                button { r#type: "button", class: "secondary-btn", onclick: move |_| oncancel.call(()), "{i18n_text.cancel}" }
                button {
                    r#type: "button",
                    class: "primary-btn",
                    disabled: !can_save,
                    onclick: move |_| onsave.call(build_source()),
                    "{i18n_text.save}"
                }
            }
        }
    }
}

/// Champ de chemin JSON avec, en dessous, la valeur qu'il donne sur le
/// premier résultat du test.
#[component]
fn PathInput(label: &'static str, placeholder: &'static str, value: Signal<String>, preview: Option<String>) -> Element {
    let mut value = value;
    rsx! {
        div { class: "item-form-field",
            label { "{label}" }
            input {
                r#type: "text",
                class: "mono",
                placeholder: "{placeholder}",
                value: "{value}",
                oninput: move |e| value.set(e.value()),
            }
            if let Some(preview) = preview {
                span { class: "path-preview", "→ {preview}" }
            }
        }
    }
}

/// Formulaire d'un niveau de la liste d'éléments d'une source (saisons,
/// épisodes, chapitres...). Modifie directement `levels[index]`.
#[component]
fn ListLevelEditor(levels: Signal<Vec<SourceListLevel>>, index: usize) -> Element {
    let i18n_text = use_texts();
    let mut levels = levels;
    let mut new_field_label = use_signal(String::new);
    let mut new_field_path = use_signal(String::new);

    let level_count = levels.read().len();
    let Some(level) = levels.read().get(index).cloned() else {
        return rsx! {};
    };
    let is_last = index + 1 == level_count;

    let pagination_value = match level.pagination {
        ListPagination::None => "none",
        ListPagination::Page { .. } => "page",
        ListPagination::Offset { .. } => "offset",
    };
    let leaf_kind_value = if level.leaf_kind == ProgressNodeKind::Card { "card" } else { "text" };

    rsx! {
        div { class: "source-list-level",
            div { class: "source-list-level-header",
                span { class: "editor-level-badge", {(i18n_text.list_level_number)(index + 1)} }
                input {
                    r#type: "text",
                    placeholder: "{i18n_text.list_level_label_placeholder}",
                    value: "{level.label}",
                    oninput: move |e| update_level(levels, index, |l| l.label = e.value()),
                }
                button {
                    r#type: "button",
                    class: "icon-btn",
                    disabled: index == 0,
                    onclick: move |_| {
                        if index > 0 {
                            levels.write().swap(index - 1, index);
                        }
                    },
                    "↑"
                }
                button {
                    r#type: "button",
                    class: "icon-btn",
                    disabled: is_last,
                    onclick: move |_| {
                        if index + 1 < levels.read().len() {
                            levels.write().swap(index, index + 1);
                        }
                    },
                    "↓"
                }
                button {
                    r#type: "button",
                    class: "icon-btn danger",
                    onclick: move |_| {
                        levels.write().remove(index);
                    },
                    "×"
                }
            }
            div { class: "item-form-field",
                label { "{i18n_text.list_url_label}" }
                input {
                    r#type: "text",
                    class: "mono",
                    placeholder: "https://api.jikan.moe/v4/anime/{{root.mal_id}}/episodes?page={{page}}",
                    value: "{level.url_template}",
                    oninput: move |e| update_level(levels, index, |l| l.url_template = e.value()),
                }
            }
            div { class: "item-form-row",
                div { class: "item-form-field",
                    label { "{i18n_text.list_items_path_label}" }
                    input {
                        r#type: "text",
                        class: "mono",
                        placeholder: "data",
                        value: "{level.items_path}",
                        oninput: move |e| update_level(levels, index, |l| l.items_path = e.value()),
                    }
                }
                div { class: "item-form-field",
                    label { "{i18n_text.list_title_label}" }
                    input {
                        r#type: "text",
                        class: "mono",
                        placeholder: "{{number}}. {{title}}",
                        value: "{level.title_path}",
                        oninput: move |e| update_level(levels, index, |l| l.title_path = e.value()),
                    }
                }
            }
            div { class: "item-form-row",
                div { class: "item-form-field",
                    label { "{i18n_text.list_image_label}" }
                    input {
                        r#type: "text",
                        class: "mono",
                        placeholder: "images.jpg.image_url",
                        value: "{level.image_path}",
                        oninput: move |e| update_level(levels, index, |l| l.image_path = e.value()),
                    }
                }
                div { class: "item-form-field",
                    label { "{i18n_text.leaf_kind_label}" }
                    if is_last {
                        select {
                            value: "{leaf_kind_value}",
                            onchange: move |e| {
                                let kind = if e.value() == "card" { ProgressNodeKind::Card } else { ProgressNodeKind::Text };
                                update_level(levels, index, |l| l.leaf_kind = kind);
                            },
                            option { value: "card", selected: leaf_kind_value == "card", "{i18n_text.leaf_kind_card}" }
                            option { value: "text", selected: leaf_kind_value == "text", "{i18n_text.leaf_kind_text}" }
                        }
                    } else {
                        span { class: "field-hint", "{i18n_text.sublevel_hint}" }
                    }
                }
            }

            label { "{i18n_text.list_fields_title}" }
            div { class: "editor-chips",
                for (field_index , field) in level.fields.iter().cloned().enumerate() {
                    span { key: "{field_index}", class: "editor-chip",
                        strong { "{field.label}" }
                        code { "{field.json_path}" }
                        button {
                            r#type: "button",
                            class: "editor-chip-remove",
                            onclick: move |_| {
                                update_level(levels, index, |l| {
                                    if field_index < l.fields.len() {
                                        l.fields.remove(field_index);
                                    }
                                });
                            },
                            "×"
                        }
                    }
                }
            }
            div { class: "editor-add-row",
                input {
                    r#type: "text",
                    placeholder: "{i18n_text.extra_field_label_placeholder}",
                    value: "{new_field_label}",
                    oninput: move |e| new_field_label.set(e.value()),
                }
                input {
                    r#type: "text",
                    class: "mono",
                    placeholder: "{i18n_text.json_path_placeholder}",
                    value: "{new_field_path}",
                    oninput: move |e| new_field_path.set(e.value()),
                }
                button {
                    r#type: "button",
                    class: "secondary-btn",
                    onclick: move |_| {
                        let label = new_field_label.read().trim().to_string();
                        let json_path = new_field_path.read().trim().to_string();
                        if !label.is_empty() && !json_path.is_empty() {
                            update_level(levels, index, |l| l.fields.push(SourceField { label, json_path }));
                            new_field_label.set(String::new());
                            new_field_path.set(String::new());
                        }
                    },
                    "{i18n_text.add_field}"
                }
            }

            details { class: "editor-advanced",
                summary { "{i18n_text.advanced_options}" }
                div { class: "item-form-row",
                    div { class: "item-form-field",
                        label { "{i18n_text.pagination_label}" }
                        select {
                            value: "{pagination_value}",
                            onchange: move |e| {
                                let pagination = match e.value().as_str() {
                                    "page" => ListPagination::Page { first: 1 },
                                    "offset" => ListPagination::Offset { page_size: 100 },
                                    _ => ListPagination::None,
                                };
                                update_level(levels, index, |l| l.pagination = pagination);
                            },
                            option { value: "none", selected: pagination_value == "none", "{i18n_text.pagination_none}" }
                            option { value: "page", selected: pagination_value == "page", "{i18n_text.pagination_page}" }
                            option { value: "offset", selected: pagination_value == "offset", "{i18n_text.pagination_offset}" }
                        }
                    }
                    if let ListPagination::Page { first } = level.pagination {
                        div { class: "item-form-field",
                            label { "{i18n_text.pagination_first_page}" }
                            input {
                                r#type: "number",
                                min: "0",
                                value: "{first}",
                                oninput: move |e| {
                                    if let Ok(first) = e.value().parse::<usize>() {
                                        update_level(levels, index, |l| l.pagination = ListPagination::Page { first });
                                    }
                                },
                            }
                        }
                    }
                    if let ListPagination::Offset { page_size } = level.pagination {
                        div { class: "item-form-field",
                            label { "{i18n_text.pagination_page_size}" }
                            input {
                                r#type: "number",
                                min: "1",
                                value: "{page_size}",
                                oninput: move |e| {
                                    if let Ok(page_size) = e.value().parse::<usize>() {
                                        let page_size = page_size.max(1);
                                        update_level(levels, index, |l| l.pagination = ListPagination::Offset { page_size });
                                    }
                                },
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Modifie `levels[index]` s'il existe encore.
fn update_level(mut levels: Signal<Vec<SourceListLevel>>, index: usize, change: impl FnOnce(&mut SourceListLevel)) {
    if let Some(level) = levels.write().get_mut(index) {
        change(level);
    }
}

/// Nombre maximum de champs listés dans l'explorateur.
const MAX_EXPLORER_ROWS: usize = 80;

/// Tous les chemins vers une valeur simple (texte, nombre...) d'un
/// résultat, avec un aperçu de la valeur. Pour un tableau, seul le premier
/// élément est parcouru (`genres[0].name`).
fn json_leaves(value: &Value) -> Vec<(String, String)> {
    fn walk(value: &Value, path: String, out: &mut Vec<(String, String)>) {
        if out.len() >= MAX_EXPLORER_ROWS {
            return;
        }
        match value {
            Value::Object(map) => {
                for (key, child) in map {
                    let child_path = if path.is_empty() { key.clone() } else { format!("{path}.{key}") };
                    walk(child, child_path, out);
                }
            }
            Value::Array(items) => {
                if let Some(first) = items.first() {
                    walk(first, format!("{path}[0]"), out);
                }
            }
            Value::Null => {}
            other => {
                let text = net::value_text(other).unwrap_or_default();
                let mut preview: String = text.chars().take(60).collect();
                if text.chars().count() > 60 {
                    preview.push('…');
                }
                out.push((path, preview));
            }
        }
    }
    let mut out = Vec::new();
    walk(value, String::new(), &mut out);
    out
}

/// Chemins des tableaux d'objets de la réponse (candidats pour le chemin
/// des résultats), sur quelques niveaux de profondeur.
fn array_paths(body: &Value) -> Vec<String> {
    fn walk(value: &Value, path: String, depth: usize, out: &mut Vec<String>) {
        match value {
            Value::Array(items) if items.first().is_some_and(Value::is_object) => out.push(path),
            Value::Object(map) if depth < 3 => {
                for (key, child) in map {
                    let child_path = if path.is_empty() { key.clone() } else { format!("{path}.{key}") };
                    walk(child, child_path, depth + 1, out);
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    walk(body, String::new(), 0, &mut out);
    out
}

/// Pour chaque niveau : son nom, le nombre d'éléments extraits et les
/// premiers titres, pour l'aperçu du test.
fn list_preview_rows(levels: &[SourceListLevel], nodes: &[ProgressNode]) -> Vec<(String, usize, String)> {
    // Chaque parent est créé avant ses enfants : la profondeur (= l'indice
    // du niveau) se calcule en une seule passe.
    let mut depths: HashMap<usize, usize> = HashMap::new();
    for node in nodes {
        let depth = node.parent_id.and_then(|p| depths.get(&p)).map(|d| d + 1).unwrap_or(0);
        depths.insert(node.id, depth);
    }

    levels
        .iter()
        .enumerate()
        .map(|(level_index, level)| {
            let titles: Vec<&str> = nodes
                .iter()
                .filter(|n| depths.get(&n.id) == Some(&level_index))
                .map(|n| n.title.as_str())
                .collect();
            let mut first_titles = titles.iter().take(3).copied().collect::<Vec<_>>().join(", ");
            if titles.len() > 3 {
                first_titles.push_str(", …");
            }
            (level.label.clone(), titles.len(), first_titles)
        })
        .collect()
}
