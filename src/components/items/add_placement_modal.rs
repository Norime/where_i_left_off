use dioxus::prelude::*;
use crate::components::Modal;
use crate::i18n::use_texts;
use crate::state::{CategoryStore, ItemStore};

/// Fenêtre pour rattacher un élément à une catégorie/sous-catégorie
/// supplémentaire : une liste déroulante de catégories (uniquement celles
/// qui ont au moins une vraie sous-catégorie), puis une seconde liste avec
/// les sous-catégories de la catégorie choisie.
#[component]
pub fn AddPlacementModal(item_id: usize, onclose: EventHandler<()>) -> Element {
    let i18n_text = use_texts();
    let mut item_store = use_context::<Signal<ItemStore>>();
    let category_store = use_context::<Signal<CategoryStore>>();

    let mut selected_category = use_signal(|| None::<usize>);
    let mut selected_subcategory = use_signal(|| None::<usize>);
    let mut error = use_signal(|| None::<&'static str>);

    let available_categories: Vec<(usize, String)> = category_store.read().categories.iter()
        .filter(|c| c.subcategories.iter().any(|s| s.id != 0))
        .map(|c| (c.id, c.name.clone()))
        .collect();

    let available_subcategories: Vec<(usize, String)> = match selected_category() {
        Some(cat_id) => category_store.read().categories.iter()
            .find(|c| c.id == cat_id)
            .map(|c| c.subcategories.iter().filter(|s| s.id != 0).map(|s| (s.id, s.name.clone())).collect())
            .unwrap_or_default(),
        None => Vec::new(),
    };

    let category_value = selected_category().map(|id| id.to_string()).unwrap_or_default();
    let subcategory_value = selected_subcategory().map(|id| id.to_string()).unwrap_or_default();

    rsx! {
        Modal { onclose: move |_| onclose.call(()),
            h2 { "{i18n_text.add_to_category}" }
            div { class: "placement-picker-form",
                label {
                    "{i18n_text.category}"
                    select {
                        value: "{category_value}",
                        onchange: move |event| {
                            match event.value().parse::<usize>() {
                                Ok(id) => selected_category.set(Some(id)),
                                Err(_) => selected_category.set(None),
                            }
                            selected_subcategory.set(None);
                        },
                        option { value: "", "{i18n_text.choose_category}" }
                        for (id , name) in available_categories.iter() {
                            option { key: "{id}", value: "{id}", "{name}" }
                        }
                    }
                }
                label {
                    "{i18n_text.subcategory}"
                    select {
                        value: "{subcategory_value}",
                        disabled: selected_category().is_none(),
                        onchange: move |event| {
                            match event.value().parse::<usize>() {
                                Ok(id) => selected_subcategory.set(Some(id)),
                                Err(_) => selected_subcategory.set(None),
                            }
                        },
                        option { value: "", "{i18n_text.choose_subcategory}" }
                        for (id , name) in available_subcategories.iter() {
                            option { key: "{id}", value: "{id}", "{name}" }
                        }
                    }
                }
                if let Some(message) = error() {
                    p { class: "auth-error", "{message}" }
                }
                div { class: "modal-actions",
                    button { r#type: "button", onclick: move |_| onclose.call(()), "{i18n_text.cancel}" }
                    button {
                        r#type: "button",
                        class: "primary-btn",
                        onclick: move |_| {
                            match (selected_category(), selected_subcategory()) {
                                (Some(cat), Some(sub)) => {
                                    item_store.write().add_placement(item_id, cat, sub);
                                    onclose.call(());
                                }
                                _ => error.set(Some(i18n_text.choose_category_and_subcategory)),
                            }
                        },
                        "{i18n_text.add}"
                    }
                }
            }
        }
    }
}