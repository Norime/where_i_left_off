use std::collections::HashSet;

use dioxus::prelude::*;

use crate::components::{FetchedItemData, InlineAddForm, ItemSourceSearch, Modal, TypeCheckboxList};
use crate::models::{ItemInfoField, ItemOrientation, ItemPlacement, ItemStatus, WorkStatus};
use crate::state::{ItemStore, TypeStore};

#[component]
pub fn NewItemModal(category_id: usize, subcategory_id: usize, onclose: EventHandler<()>) -> Element {
    let mut item_store = use_context::<Signal<ItemStore>>();
    let mut type_store = use_context::<Signal<TypeStore>>();

    let mut title = use_signal(String::new);
    let mut image_url = use_signal(|| Option::<String>::None);
    let mut description = use_signal(String::new);
    let mut orientation = use_signal(|| ItemOrientation::Vertical);
    let mut selected_type_ids = use_signal(HashSet::<usize>::new);
    let mut extra_info = use_signal(Vec::<ItemInfoField>::new);

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

                    let placement = ItemPlacement {
                        category_id,
                        subcategory_id,
                        status: ItemStatus::New,
                        type_ids: selected_type_ids.read().iter().cloned().collect(),
                        progress: Vec::new(),
                        work_status: WorkStatus::default(),
                        tabs: Vec::new(),
                    };

                    item_store
                        .write()
                        .add_item(
                            title.read().trim().to_string(),
                            description.read().clone(),
                            placement,
                            *orientation.read(),
                            image_url.read().clone(),
                            extra_info.read().clone(),
                        );
                    onclose.call(());
                },

                h2 { "Nouvel élément" }

                ItemSourceSearch {
                    scopes: vec![(category_id, subcategory_id)],
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
                        label { "Titre" }
                        input {
                            r#type: "text",
                            value: "{title}",
                            oninput: move |event| title.set(event.value()),
                        }
                    }
                    div { class: "item-form-field",
                        label { "URL de l'image" }
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
                    label { "Description" }
                    textarea {
                        class: "item-form-textarea",
                        value: "{description}",
                        oninput: move |event| description.set(event.value()),
                    }
                }

                div { class: "item-form-row",
                    div { class: "item-form-field",
                        label { "Type" }
                        TypeCheckboxList {
                            category_ids: vec![category_id],
                            selected: selected_type_ids,
                        }
                        InlineAddForm {
                            placeholder: "Nouveau type",
                            onsubmit: move |name: String| {
                                let new_id = type_store.write().add_type(category_id, name);
                                selected_type_ids.write().insert(new_id);
                            },
                        }
                    }
                    div { class: "item-form-field",
                        label { "Format" }
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
                            option { value: "vertical", "Vertical" }
                            option { value: "horizontal", "Horizontal" }
                        }
                    }
                }

                div { class: "modal-actions",
                    button {
                        r#type: "button",
                        class: "secondary-btn",
                        onclick: move |_| onclose.call(()),
                        "Annuler"
                    }
                    button {
                        r#type: "submit",
                        class: "primary-btn",
                        disabled: !can_save,
                        "Créer"
                    }
                }
            }
        }
    }
}