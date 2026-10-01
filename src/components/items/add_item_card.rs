use dioxus::prelude::*;
use crate::components::NewItemModal;

/// Carte "+" qui ouvre la fenêtre de création d'un élément au clic.
#[component]
pub fn AddItemCard(category_id: usize, subcategory_id: usize) -> Element {
    let mut show_modal = use_signal(|| false);

    rsx! {
        button {
            class: "item-card item-card--add",
            onclick: move |_| show_modal.set(true),
            span { class: "add-icon", "+" }
        }
        if show_modal() {
            NewItemModal {
                category_id,
                subcategory_id,
                onclose: move |_| show_modal.set(false),
            }
        }
    }
}