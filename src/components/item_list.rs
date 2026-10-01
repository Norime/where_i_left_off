use dioxus::prelude::*;
use crate::models::sample_items;
use crate::components::ItemCard;

#[component]
pub fn ItemList() -> Element {
    let items = use_signal(sample_items);

    rsx! {
        div { class: "item-grid",
            for item in items.read().iter() {
                ItemCard { key: "{item.id}", item: item.clone() }
            }
        }
    }
}