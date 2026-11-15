use dioxus::prelude::*;

use crate::i18n::use_texts;
use crate::models::ItemInfoField;
use crate::state::ItemStore;

/// Infos globales de l'œuvre (studio, plateforme, date de sortie…),
/// partagées par toutes ses catégories : valeur modifiable sur place,
/// suppression, ajout. Remplies par une source ou proposées vides par le
/// modèle de la sous-catégorie.
#[component]
pub fn ItemInfoPanel(item_id: usize, fields: Vec<ItemInfoField>) -> Element {
    let i18n_text = use_texts();
    let mut item_store = use_context::<Signal<ItemStore>>();
    let mut new_label = use_signal(String::new);
    let mut new_value = use_signal(String::new);

    // Relit toujours les infos actuelles avant de les modifier.
    let mut edit = move |change: &dyn Fn(&mut Vec<ItemInfoField>)| {
        let mut current = item_store
            .read()
            .items
            .iter()
            .find(|i| i.id == item_id)
            .map(|i| i.extra_info.clone())
            .unwrap_or_default();
        change(&mut current);
        item_store.write().set_item_info(item_id, current);
    };

    let mut add = move || {
        let label = new_label.read().trim().to_string();
        if label.is_empty() {
            return;
        }
        let value = new_value.read().trim().to_string();
        edit(&|fields| fields.push(ItemInfoField { label: label.clone(), value: value.clone() }));
        new_label.set(String::new());
        new_value.set(String::new());
    };

    rsx! {
        section { class: "item-info-panel",
            h3 { "{i18n_text.info_title}" }
            if fields.is_empty() {
                p { class: "progress-usage-hint", "{i18n_text.info_none}" }
            }
            div { class: "item-info-grid",
                for (index , field) in fields.into_iter().enumerate() {
                    // Ligne sans boîte à elle : ses cellules se placent dans la grille.
                    div { key: "{index}", class: "item-info-row",
                        span { class: "item-info-label", "{field.label}" }
                        input {
                            r#type: "text",
                            class: "item-info-value",
                            value: "{field.value}",
                            placeholder: "{i18n_text.info_empty_value}",
                            onchange: move |event| {
                                let value = event.value().trim().to_string();
                                edit(&|fields| {
                                    if let Some(field) = fields.get_mut(index) {
                                        field.value = value.clone();
                                    }
                                });
                            },
                        }
                        button {
                            r#type: "button",
                            class: "item-info-remove",
                            title: "{i18n_text.delete}",
                            onclick: move |_| {
                                edit(&|fields| {
                                    if index < fields.len() {
                                        fields.remove(index);
                                    }
                                });
                            },
                            "×"
                        }
                    }
                }
            }
            div { class: "item-info-add",
                input {
                    r#type: "text",
                    placeholder: "{i18n_text.info_add_label}",
                    value: "{new_label}",
                    oninput: move |event| new_label.set(event.value()),
                }
                input {
                    r#type: "text",
                    placeholder: "{i18n_text.info_add_value}",
                    value: "{new_value}",
                    oninput: move |event| new_value.set(event.value()),
                    onkeydown: move |event| {
                        if event.key() == Key::Enter {
                            add();
                        }
                    },
                }
                button { r#type: "button", class: "secondary-btn", onclick: move |_| add(), "+" }
            }
        }
    }
}
