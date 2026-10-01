use dioxus::prelude::*;
use crate::components::DeleteButton;
use crate::models::{Item, ItemOrientation};
use crate::routes::Route;
use crate::state::ItemStore;

/// Une carte cliquable : une image (ou un fond gris par défaut) avec le
/// titre affiché en bas, dans un format vertical ou horizontal selon
/// `item.orientation`.
///
/// `origin_category_id`/`origin_subcategory_id` indiquent d'où vient la
/// carte (la catégorie/sous-catégorie affichée dans `CategoryView`), pour
/// que `ItemDetail` puisse rouvrir directement sur ce placement. `0` = pas
/// de catégorie/sous-catégorie précise (valeur par défaut si on n'indique
/// rien, ex: si `ItemCard` est utilisée ailleurs sans ces props).
#[component]
pub fn ItemCard(
    item: Item,
    #[props(default)] origin_category_id: usize,
    #[props(default)] origin_subcategory_id: usize,
) -> Element {
    let mut item_store = use_context::<Signal<ItemStore>>();
    let item_id = item.id;

    let orientation_class = match item.orientation {
        ItemOrientation::Vertical => "item-card--vertical",
        ItemOrientation::Horizontal => "item-card--horizontal",
    };

    let background_style = item.image_url
        .as_ref()
        .map(|url| format!("background-image: url('{url}');"))
        .unwrap_or_default();

    rsx! {
        Link {
            to: Route::ItemDetail {
                id: item_id,
                category_id: origin_category_id,
                subcategory_id: origin_subcategory_id,
            },
            class: "item-card {orientation_class}",
            style: "{background_style}",
            div { class: "item-card-title", "{item.title}" }
            div {
                class: "item-card-delete-wrapper",
                onclick: move |event| event.stop_propagation(),
                DeleteButton {
                    onconfirm: move |_| {
                        item_store.write().remove_item(item_id);
                    },
                }
            }
        }
    }
}