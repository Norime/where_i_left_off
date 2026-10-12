use std::collections::HashMap;

use dioxus::prelude::*;

use crate::components::{DeleteButton, Modal};
use crate::i18n::use_texts;
use crate::json_path;
use crate::models::{ApiSource, ListPagination, ProgressNode, ProgressNodeKind, SourceField, SourceListLevel, SourceScope};
use crate::net;
use crate::source_import;
use crate::state::{CategoryStore, SourceStore};

#[component]
pub fn ManageSourcesModal(category_id: usize, subcategory_id: usize, oncancel: EventHandler<()>) -> Element {
    let i18n_text = use_texts();
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
                h2 { "{i18n_text.sources_for_subcategory}" }
                p { class: "toolbar-hint",
                    "{i18n_text.sources_intro}"
                }

                h3 { "{i18n_text.linked_here}" }
                if linked_sources.is_empty() {
                    p { class: "toolbar-hint",
                        "{i18n_text.no_linked_source}"
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
                                    "{i18n_text.edit}"
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
                                    list_levels: Vec::new(),
                                }),
                            );
                    },
                    "{i18n_text.new_source}"
                }

                div { class: "manage-sources-columns",
                    div { class: "manage-sources-column",
                        h3 { "{i18n_text.in_my_category}" }
                        if same_category_candidates.is_empty() {
                            p { class: "toolbar-hint", "{i18n_text.no_other_source_in_category}" }
                        }
                        ul { class: "source-list",
                            for source in same_category_candidates.iter().cloned() {
                                li {
                                    key: "{source.id}",
                                    class: "source-list-item",
                                    div {
                                        span { "{source.name}" }
                                        p { class: "source-usage-hint",
                                            {(i18n_text.used_in)(&describe(&source.linked_scopes))}
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
                                        "{i18n_text.link}"
                                    }
                                }
                            }
                        }
                    }
                    div { class: "manage-sources-column",
                        h3 { "{i18n_text.other_categories}" }
                        if other_category_candidates.is_empty() {
                            p { class: "toolbar-hint", "{i18n_text.no_source_in_other_category}" }
                        }
                        ul { class: "source-list",
                            for source in other_category_candidates.iter().cloned() {
                                li {
                                    key: "{source.id}",
                                    class: "source-list-item",
                                    div {
                                        span { "{source.name}" }
                                        p { class: "source-usage-hint",
                                            {(i18n_text.used_in)(&describe(&source.linked_scopes))}
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
                                        "{i18n_text.link}"
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
    let i18n_text = use_texts();
    let mut name = use_signal(|| draft.name.clone());
    let mut search_url_template = use_signal(|| draft.search_url_template.clone());
    let mut results_path = use_signal(|| draft.results_path.clone());
    let mut title_path = use_signal(|| draft.title_path.clone());
    let mut image_path = use_signal(|| draft.image_path.clone());
    let mut description_path = use_signal(|| draft.description_path.clone());
    let mut extra_fields = use_signal(|| draft.extra_fields.clone());
    let mut list_levels = use_signal(|| draft.list_levels.clone());

    let mut new_field_label = use_signal(String::new);
    let mut new_field_path = use_signal(String::new);

    let mut test_query = use_signal(String::new);
    let mut test_raw_result = use_signal(|| Option::<serde_json::Value>::None);
    let mut test_error = use_signal(|| Option::<net::SearchError>::None);
    let mut is_testing = use_signal(|| false);
    let mut test_progress = use_signal(|| Option::<Vec<ProgressNode>>::None);
    let mut test_list_error = use_signal(|| Option::<net::SearchError>::None);

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
        .unwrap_or_else(|| i18n_text.nothing.to_string());
    let preview_image_text = raw_result
        .as_ref()
        .and_then(|v| json_path::resolve_as_string(v, &image_path.read()))
        .unwrap_or_else(|| i18n_text.nothing.to_string());
    let preview_description_text = raw_result
        .as_ref()
        .and_then(|v| json_path::resolve_as_string(v, &description_path.read()))
        .unwrap_or_else(|| i18n_text.nothing.to_string());
    let has_test_result = raw_result.is_some();
    let level_count = list_levels.read().len();
    let list_preview: Option<Vec<(String, usize, String)>> = test_progress
        .read()
        .as_ref()
        .map(|nodes| list_preview_rows(&list_levels.read(), nodes));

    rsx! {
        div { class: "source-editor",
            if is_shared {
                p { class: "form-error",
                    "{i18n_text.shared_source_warning}"
                }
            }
            div { class: "item-form-field",
                label { "{i18n_text.source_name}" }
                input {
                    r#type: "text",
                    value: "{name}",
                    oninput: move |e| name.set(e.value()),
                }
            }
            div { class: "item-form-field",
                label { "{i18n_text.search_url_label}" }
                input {
                    r#type: "text",
                    placeholder: "https://api.jikan.moe/v4/anime?q={{query}}",
                    value: "{search_url_template}",
                    oninput: move |e| search_url_template.set(e.value()),
                }
            }
            div { class: "item-form-row",
                div { class: "item-form-field",
                    label { "{i18n_text.results_path_label}" }
                    input {
                        r#type: "text",
                        placeholder: "data",
                        value: "{results_path}",
                        oninput: move |e| results_path.set(e.value()),
                    }
                }
                div { class: "item-form-field",
                    label { "{i18n_text.title_path_label}" }
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
                    label { "{i18n_text.image_path_label}" }
                    input {
                        r#type: "text",
                        placeholder: "images.jpg.image_url",
                        value: "{image_path}",
                        oninput: move |e| image_path.set(e.value()),
                    }
                }
                div { class: "item-form-field",
                    label { "{i18n_text.description_path_label}" }
                    input {
                        r#type: "text",
                        placeholder: "synopsis",
                        value: "{description_path}",
                        oninput: move |e| description_path.set(e.value()),
                    }
                }
            }

            h3 { "{i18n_text.extra_fields_title}" }
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
                    placeholder: "{i18n_text.extra_field_label_placeholder}",
                    value: "{new_field_label}",
                    oninput: move |e| new_field_label.set(e.value()),
                }
                input {
                    r#type: "text",
                    placeholder: "{i18n_text.json_path_placeholder}",
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
                    "{i18n_text.add_field}"
                }
            }

            h3 { "{i18n_text.list_levels_title}" }
            p { class: "toolbar-hint", "{i18n_text.list_levels_hint}" }
            for index in 0..level_count {
                ListLevelEditor { key: "{index}", levels: list_levels, index }
            }
            button {
                r#type: "button",
                class: "secondary-btn",
                onclick: move |_| {
                    list_levels
                        .write()
                        .push(SourceListLevel {
                            leaf_kind: ProgressNodeKind::Card,
                            ..Default::default()
                        });
                },
                "{i18n_text.add_list_level}"
            }

            h3 { "{i18n_text.test_title}" }
            p { class: "toolbar-hint",
                "{i18n_text.test_hint}"
            }
            div { class: "item-form-row",
                input {
                    r#type: "text",
                    placeholder: "{i18n_text.test_query_placeholder}",
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
                                list_levels: list_levels.read().clone(),
                            };
                            let query = test_query.read().clone();
                            if query.trim().is_empty() {
                                return;
                            }
                            is_testing.set(true);
                            test_error.set(None);
                            test_list_error.set(None);
                            test_progress.set(None);
                            spawn(async move {
                                match net::search(&candidate, &query).await {
                                    Ok(found) => {
                                        let first = found.into_iter().next();
                                        test_raw_result.set(first.clone());
                                        // Une seule page par liste : on vérifie les chemins,
                                        // pas besoin de tout télécharger.
                                        if let Some(first) = first.filter(|_| !candidate.list_levels.is_empty()) {
                                            match source_import::fetch_progress(&candidate, &first, 1).await {
                                                Ok(nodes) => test_progress.set(Some(nodes)),
                                                Err(e) => test_list_error.set(Some(e)),
                                            }
                                        }
                                    }
                                    Err(e) => test_error.set(Some(e)),
                                }
                                is_testing.set(false);
                            });
                        }
                    },
                    if *is_testing.read() {
                        "{i18n_text.testing}"
                    } else {
                        "{i18n_text.run_test}"
                    }
                }
            }

            if let Some(error) = test_error.read().clone() {
                p { class: "form-error", {i18n_text.search_error(&error)} }
            }

            if has_test_result {
                div { class: "source-test-preview",
                    p {
                        "{i18n_text.extracted_title}"
                        strong { "{preview_title_text}" }
                    }
                    p {
                        "{i18n_text.extracted_image}"
                        strong { "{preview_image_text}" }
                    }
                    p {
                        "{i18n_text.extracted_description}"
                        strong { "{preview_description_text}" }
                    }
                    if let Some(rows) = list_preview {
                        p { "{i18n_text.extracted_list}" }
                        ul { class: "source-field-list",
                            for (index , (label , count , first_titles)) in rows.into_iter().enumerate() {
                                li { key: "{index}",
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
            }

            div { class: "modal-actions",
                button {
                    r#type: "button",
                    class: "secondary-btn",
                    onclick: move |_| oncancel.call(()),
                    "{i18n_text.cancel}"
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
                                    list_levels: list_levels.read().clone(),
                                });
                        }
                    },
                    "{i18n_text.save}"
                }
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

    let field_rows: Vec<(usize, String, String)> = level
        .fields
        .iter()
        .cloned()
        .enumerate()
        .map(|(field_index, field)| (field_index, field.label, field.json_path))
        .collect();

    let pagination_value = match level.pagination {
        ListPagination::None => "none",
        ListPagination::Page { .. } => "page",
        ListPagination::Offset { .. } => "offset",
    };
    let leaf_kind_value = if level.leaf_kind == ProgressNodeKind::Card { "card" } else { "text" };

    rsx! {
        div { class: "source-list-level",
            div { class: "source-list-level-header",
                strong { {(i18n_text.list_level_number)(index + 1)} }
                input {
                    r#type: "text",
                    placeholder: "{i18n_text.list_level_label_placeholder}",
                    value: "{level.label}",
                    oninput: move |e| update_level(levels, index, |l| l.label = e.value()),
                }
                button {
                    r#type: "button",
                    class: "secondary-btn",
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
                    class: "secondary-btn",
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
                    class: "danger-btn",
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
                    placeholder: "https://api.jikan.moe/v4/anime/{{mal_id}}/episodes?page={{page}}",
                    value: "{level.url_template}",
                    oninput: move |e| update_level(levels, index, |l| l.url_template = e.value()),
                }
            }
            div { class: "item-form-row",
                div { class: "item-form-field",
                    label { "{i18n_text.list_items_path_label}" }
                    input {
                        r#type: "text",
                        placeholder: "data",
                        value: "{level.items_path}",
                        oninput: move |e| update_level(levels, index, |l| l.items_path = e.value()),
                    }
                }
                div { class: "item-form-field",
                    label { "{i18n_text.list_title_label}" }
                    input {
                        r#type: "text",
                        placeholder: "title",
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
                                let kind = if e.value() == "card" {
                                    ProgressNodeKind::Card
                                } else {
                                    ProgressNodeKind::Text
                                };
                                update_level(levels, index, |l| l.leaf_kind = kind);
                            },
                            option { value: "card", "{i18n_text.leaf_kind_card}" }
                            option { value: "text", "{i18n_text.leaf_kind_text}" }
                        }
                    } else {
                        p { class: "toolbar-hint", "{i18n_text.sublevel_hint}" }
                    }
                }
            }
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
                        option { value: "none", "{i18n_text.pagination_none}" }
                        option { value: "page", "{i18n_text.pagination_page}" }
                        option { value: "offset", "{i18n_text.pagination_offset}" }
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
                                    update_level(
                                        levels,
                                        index,
                                        |l| l.pagination = ListPagination::Offset { page_size },
                                    );
                                }
                            },
                        }
                    }
                }
            }

            label { "{i18n_text.list_fields_title}" }
            ul { class: "source-field-list",
                for (field_index , label , path) in field_rows {
                    li { key: "{field_index}",
                        span { "{label} → {path}" }
                        button {
                            r#type: "button",
                            class: "danger-btn",
                            onclick: move |_| {
                                update_level(
                                    levels,
                                    index,
                                    |l| {
                                        if field_index < l.fields.len() {
                                            l.fields.remove(field_index);
                                        }
                                    },
                                );
                            },
                            "×"
                        }
                    }
                }
            }
            div { class: "item-form-row",
                input {
                    r#type: "text",
                    placeholder: "{i18n_text.extra_field_label_placeholder}",
                    value: "{new_field_label}",
                    oninput: move |e| new_field_label.set(e.value()),
                }
                input {
                    r#type: "text",
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
        }
    }
}

/// Modifie `levels[index]` s'il existe encore.
fn update_level(mut levels: Signal<Vec<SourceListLevel>>, index: usize, change: impl FnOnce(&mut SourceListLevel)) {
    if let Some(level) = levels.write().get_mut(index) {
        change(level);
    }
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
