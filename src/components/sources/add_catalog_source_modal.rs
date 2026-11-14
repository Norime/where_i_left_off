use dioxus::prelude::*;

use crate::components::Modal;
use crate::i18n::{use_texts, Language};
use crate::models::SourceScope;
use crate::source_catalog;
use crate::state::{CategoryStore, SourceStore};

/// Ajoute une source du catalogue à une sous-catégorie. Demande la
/// sous-catégorie (sauf si `fixed_scope` est donné) et la clé d'API si la
/// source en exige une. Si cette source existe déjà (même entrée, même
/// langue), elle est simplement reliée à la sous-catégorie choisie.
#[component]
pub fn AddCatalogSourceModal(
    entry_id: &'static str,
    content_language: Language,
    fixed_scope: Option<SourceScope>,
    onclose: EventHandler<()>,
) -> Element {
    let i18n_text = use_texts();
    let ui_language = use_context::<Memo<Language>>()();
    let mut source_store = use_context::<Signal<SourceStore>>();
    let category_store = use_context::<Signal<CategoryStore>>();
    let Some(entry) = source_catalog::find(entry_id) else {
        return rsx! {};
    };

    // Sous-catégories possibles (pas « Tous », id 0), groupées par catégorie.
    let choices: Vec<(usize, String, Vec<(usize, String)>)> = category_store
        .read()
        .categories
        .iter()
        .map(|c| {
            let subs = c.subcategories.iter().filter(|s| s.id != 0).map(|s| (s.id, s.display_name(ui_language).to_string())).collect();
            (c.id, c.display_name(ui_language).to_string(), subs)
        })
        .filter(|(_, _, subs): &(usize, String, Vec<(usize, String)>)| !subs.is_empty())
        .collect();

    let first_scope = choices
        .first()
        .and_then(|(category_id, _, subs)| subs.first().map(|(sub_id, _)| SourceScope { category_id: *category_id, subcategory_id: *sub_id }));
    let mut selected = use_signal(|| fixed_scope.or(first_scope));
    let mut api_key = use_signal(String::new);

    let catalog_key = entry.catalog_key(content_language);
    let existing = source_store.read().find_catalog_source(&catalog_key).cloned();
    let needs_key = entry.api_key_url.is_some() && existing.is_none();
    let already_linked = matches!((&existing, selected()), (Some(source), Some(scope)) if source.linked_scopes.contains(&scope));
    let can_add = selected().is_some() && !already_linked && (!needs_key || !api_key.read().trim().is_empty());
    let selected_value = selected().map(|s| format!("{}:{}", s.category_id, s.subcategory_id)).unwrap_or_default();

    rsx! {
        Modal { onclose: move |_| onclose.call(()),
            div { class: "add-catalog-source",
                h2 { {(i18n_text.add_catalog_title)(&entry.name(content_language))} }
                p { class: "toolbar-hint", "{entry.description(ui_language)}" }

                if fixed_scope.is_none() {
                    if choices.is_empty() {
                        p { class: "form-error", "{i18n_text.add_catalog_no_subcategory}" }
                    } else {
                        div { class: "item-form-field",
                            label { "{i18n_text.add_catalog_where}" }
                            select {
                                value: "{selected_value}",
                                onchange: move |e| {
                                    let value = e.value();
                                    let parsed = value
                                        .split_once(':')
                                        .and_then(|(c, s)| Some(SourceScope { category_id: c.parse().ok()?, subcategory_id: s.parse().ok()? }));
                                    selected.set(parsed);
                                },
                                for (category_id , category_name , subs) in choices.iter().cloned() {
                                    optgroup { key: "{category_id}", label: "{category_name}",
                                        for (sub_id , sub_name) in subs {
                                            option {
                                                key: "{sub_id}",
                                                value: "{category_id}:{sub_id}",
                                                selected: selected_value == format!("{category_id}:{sub_id}"),
                                                "{category_name} › {sub_name}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if existing.is_some() && !already_linked {
                    p { class: "toolbar-hint", "{i18n_text.add_catalog_reuse_hint}" }
                }
                if already_linked {
                    p { class: "form-error", "{i18n_text.already_linked_here}" }
                }

                if needs_key {
                    div { class: "item-form-field",
                        label { "🔑 {i18n_text.api_key_label}" }
                        input {
                            r#type: "text",
                            value: "{api_key}",
                            oninput: move |e| api_key.set(e.value()),
                        }
                        if let Some(url) = entry.api_key_url {
                            p { class: "toolbar-hint", {(i18n_text.api_key_hint)(url)} }
                        }
                    }
                }

                div { class: "modal-actions",
                    button {
                        r#type: "button",
                        class: "secondary-btn",
                        onclick: move |_| onclose.call(()),
                        "{i18n_text.cancel}"
                    }
                    button {
                        r#type: "button",
                        class: "primary-btn",
                        disabled: !can_add,
                        onclick: move |_| {
                            let Some(scope) = selected() else { return };
                            let mut store = source_store.write();
                            match store.find_catalog_source(&catalog_key).map(|s| s.id) {
                                Some(source_id) => store.link_source(source_id, scope.category_id, scope.subcategory_id),
                                None => {
                                    let mut source = entry.build(content_language, &api_key.read());
                                    source.linked_scopes = vec![scope];
                                    store.add_source(source);
                                }
                            }
                            onclose.call(());
                        },
                        "{i18n_text.catalog_add}"
                    }
                }
            }
        }
    }
}
