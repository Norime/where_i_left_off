use dioxus::prelude::*;
use std::collections::HashSet;

use crate::components::{CategorySettingsModal, Modal};
use crate::i18n::{use_texts, Language};
use crate::models::{Category, SubcategoryTemplate};
use crate::routes::Route;
use crate::state::CategoryStore;

#[component]
pub fn CategoryTreeItem(category: Category, expanded: Signal<HashSet<usize>>) -> Element {
    let i18n_text = use_texts();
    let language = use_context::<Memo<Language>>()();
    // Élément en cours de configuration : `Some(None)` pour la catégorie,
    // `Some(Some(id))` pour une de ses sous-catégories.
    let mut editing = use_signal(|| None::<Option<usize>>);
    let category_id = category.id;
    let is_expanded = expanded.read().contains(&category_id);
    let toggle_icon = if is_expanded { "▼" } else { "▶" };

    rsx! {
        li { class: "category-item",
            div {
                class: "category-header",
                onclick: move |_| {
                    let mut set = expanded.write();
                    if set.contains(&category_id) {
                        set.remove(&category_id);
                    } else {
                        set.insert(category_id);
                    }
                },
                span { class: "category-toggle", "{toggle_icon}" }
                span { class: "category-name", "{category.display_name(language)}" }
                button {
                    class: "settings-btn",
                    title: "{i18n_text.settings}",
                    // Sans `stop_propagation`, le clic replierait/déplierait la catégorie.
                    onclick: move |evt| {
                        evt.stop_propagation();
                        editing.set(Some(None));
                    },
                    "⚙"
                }
            }
            if is_expanded {
                ul { class: "subcategory-list",
                    for (sub_id , sub_name , is_all) in category
                        .subcategories
                        .iter()
                        .map(|s| (s.id, s.display_name(language).to_string(), s.id == 0))
                        .collect::<Vec<_>>()
                    {
                        li { key: "{sub_id}", class: "subcategory-item",
                            Link {
                                to: Route::CategoryView {
                                    category_id,
                                    subcategory_id: sub_id,
                                },
                                // La sous-catégorie "toutes" est stockée sous le nom "All" :
                                // on affiche son nom dans la langue active.
                                if is_all { "{i18n_text.subcategory_all}" } else { "{sub_name}" }
                            }
                            if !is_all {
                                button {
                                    class: "settings-btn",
                                    title: "{i18n_text.settings}",
                                    onclick: move |_| editing.set(Some(Some(sub_id))),
                                    "⚙"
                                }
                            }
                        }
                    }
                    li {
                        SubcategoryAddForm { category_id }
                    }
                }
            }
            // Rendue hors de `.category-header` pour que les clics dans la
            // modale ne remontent pas jusqu'au repli de la catégorie.
            if let Some(subcategory_id) = editing() {
                CategorySettingsModal {
                    category_id,
                    subcategory_id,
                    onclose: move |_| editing.set(None),
                }
            }
        }
    }
}

/// Ajout d'une sous-catégorie : un bouton pleine largeur qui ouvre la
/// pop-up demandant son nom et son modèle.
#[component]
fn SubcategoryAddForm(category_id: usize) -> Element {
    let i18n_text = use_texts();
    let mut open = use_signal(|| false);

    rsx! {
        button {
            r#type: "button",
            class: "sidebar-add-btn",
            title: "{i18n_text.new_subcategory}",
            onclick: move |_| open.set(true),
            "+"
        }
        if open() {
            SubcategoryCreateModal { category_id, onclose: move |_| open.set(false) }
        }
    }
}

/// Pop-up de création d'une sous-catégorie : son nom, et le modèle qui
/// préréglera ses fiches (aucun par défaut, modifiable ensuite dans sa
/// configuration).
#[component]
fn SubcategoryCreateModal(category_id: usize, onclose: EventHandler<()>) -> Element {
    let i18n_text = use_texts();
    let mut category_store = use_context::<Signal<CategoryStore>>();
    let mut name = use_signal(String::new);
    let mut template = use_signal(|| None::<SubcategoryTemplate>);
    let current = template();
    let is_empty = name.read().trim().is_empty();

    rsx! {
        Modal { onclose: move |_| onclose.call(()),
            h2 { "{i18n_text.new_subcategory}" }
            form {
                class: "new-item-modal-form",
                onsubmit: move |event: FormEvent| {
                    event.prevent_default();
                    let new_name = name.read().trim().to_string();
                    if new_name.is_empty() {
                        return;
                    }
                    category_store.write().add_subcategory(category_id, new_name, template());
                    onclose.call(());
                },
                label {
                    "{i18n_text.name_label}"
                    input {
                        autofocus: true,
                        value: "{name}",
                        oninput: move |e| name.set(e.value()),
                    }
                }
                label {
                    "{i18n_text.template_label}"
                    select {
                        onchange: move |e| template.set(SubcategoryTemplate::from_key(&e.value())),
                        option { value: "", selected: current.is_none(), "{i18n_text.template_none}" }
                        for option_template in SubcategoryTemplate::ALL {
                            option {
                                key: "{option_template.key()}",
                                value: "{option_template.key()}",
                                selected: current == Some(option_template),
                                "{i18n_text.template_name(option_template)}"
                            }
                        }
                    }
                    span { class: "translation-hint", "{i18n_text.template_hint}" }
                }
                div { class: "modal-actions",
                    button {
                        r#type: "button",
                        onclick: move |_| onclose.call(()),
                        "{i18n_text.cancel}"
                    }
                    button {
                        r#type: "submit",
                        class: "primary-btn",
                        disabled: is_empty,
                        "{i18n_text.create}"
                    }
                }
            }
        }
    }
}
