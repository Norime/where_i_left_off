use dioxus::prelude::*;
use crate::state::ItemStore;

/// Formulaire d'ajout d'un élément dans une catégorie/sous-catégorie précise
/// (jamais affiché sur "All", qui n'est qu'une vue agrégée).
#[component]
pub fn NewItemForm(category_id: usize, subcategory_id: usize) -> Element {
    let mut item_store = use_context::<Signal<ItemStore>>();
    let mut title = use_signal(String::new);
    let mut description = use_signal(String::new);

    rsx! {
        form {
            class: "new-item-form",
            onsubmit: move |event| {
                event.prevent_default();
                let title_value = title.read().trim().to_string();
                if title_value.is_empty() {
                    return;
                }
                let description_value = description.read().trim().to_string();

                item_store
                    .write()
                    .add_item(category_id, subcategory_id, title_value, description_value);
                title.set(String::new());
                description.set(String::new());
            },
            input {
                placeholder: "Titre de l'élément",
                value: "{title}",
                oninput: move |event| title.set(event.value()),
            }
            input {
                placeholder: "Description",
                value: "{description}",
                oninput: move |event| description.set(event.value()),
            }
            button { r#type: "submit", "Ajouter l'élément" }
        }
    }
}