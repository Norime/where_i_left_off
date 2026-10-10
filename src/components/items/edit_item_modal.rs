use dioxus::prelude::*;

use crate::components::{FetchedItemData, ItemSourceSearch, Modal};
use crate::i18n::use_texts;
use crate::models::{Item, ItemOrientation};
use crate::state::ItemStore;

/// Édite les champs globaux de l'item (titre, image, description, format).
/// Le type et la progression sont propres à chaque catégorie/sous-catégorie
/// et se modifient directement dans le détail de l'item (bloc du bas), pas
/// ici.
#[component]
pub fn EditItemModal(item: Item, onclose: EventHandler<()>) -> Element {
    let i18n_text = use_texts();
    let mut item_store = use_context::<Signal<ItemStore>>();

    let item_scopes: Vec<(usize, usize)> = item.placements.iter().map(|p| (p.category_id, p.subcategory_id)).collect();

    let mut title = use_signal(|| item.title.clone());
    let mut image_url = use_signal(|| item.image_url.clone());
    let mut description = use_signal(|| item.description.clone());
    let mut orientation = use_signal(|| item.orientation);
    let mut extra_info = use_signal(|| item.extra_info.clone());

    let item_id = item.id;
    let can_save = !title.read().trim().is_empty();

    rsx! {
        Modal { onclose: move |_| onclose.call(()),
            form {
                class: "item-modal-form",
                onsubmit: move |event| {
                    event.prevent_default();
                    if title.read().trim().is_empty() {
                        return;
                    }

                    item_store
                        .write()
                        .update_item(
                            item_id,
                            title.read().trim().to_string(),
                            description.read().clone(),
                            *orientation.read(),
                            image_url.read().clone(),
                            extra_info.read().clone(),
                        );
                    onclose.call(());
                },

                h2 { "{i18n_text.edit_item_title}" }

                ItemSourceSearch {
                    scopes: item_scopes.clone(),
                    on_apply: move |data: FetchedItemData| {
                        title.set(data.title);
                        if data.image_url.is_some() {
                            image_url.set(data.image_url);
                        }
                        description.set(data.description);
                        extra_info.set(data.extra_info);
                    },
                }

                div { class: "item-form-row",
                    div { class: "item-form-field",
                        label { "{i18n_text.title}" }
                        input {
                            r#type: "text",
                            value: "{title}",
                            oninput: move |event| title.set(event.value()),
                        }
                    }
                    div { class: "item-form-field",
                        label { "{i18n_text.image_url}" }
                        input {
                            r#type: "text",
                            value: image_url.read().clone().unwrap_or_default(),
                            oninput: move |event| {
                                let value = event.value();
                                image_url.set(if value.trim().is_empty() { None } else { Some(value) });
                            },
                        }
                    }
                }

                div { class: "item-form-field",
                    label { "{i18n_text.description}" }
                    textarea {
                        class: "item-form-textarea",
                        value: "{description}",
                        oninput: move |event| description.set(event.value()),
                    }
                }

                div { class: "item-form-row",
                    div { class: "item-form-field",
                        label { "{i18n_text.format}" }
                        select {
                            value: match *orientation.read() {
                                ItemOrientation::Vertical => "vertical",
                                ItemOrientation::Horizontal => "horizontal",
                            },
                            onchange: move |event| {
                                orientation
                                    .set(
                                        if event.value() == "horizontal" {
                                            ItemOrientation::Horizontal
                                        } else {
                                            ItemOrientation::Vertical
                                        },
                                    );
                            },
                            option { value: "vertical", "{i18n_text.vertical}" }
                            option { value: "horizontal", "{i18n_text.horizontal}" }
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
                        r#type: "submit",
                        class: "primary-btn",
                        disabled: !can_save,
                        "{i18n_text.save}"
                    }
                }
            }
        }
    }
}