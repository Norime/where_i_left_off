use dioxus::prelude::*;
use crate::components::AppLayout;
use crate::pages::{Home, CategoryView, ItemDetail, Profile, UniverseView};

/// Toutes les routes de l'application, une fois connecté.
///
/// `ItemDetail` reçoit en plus `category_id`/`subcategory_id` en query
/// params (ex: `/item/12?category_id=1&subcategory_id=3`), pour rouvrir le
/// détail directement sur le bon placement quand on vient d'une catégorie
/// précise. `0` veut dire "pas de catégorie/sous-catégorie précisée" (même
/// convention que `subcategory_id` sur la route `CategoryView` ci-dessous).
#[derive(Clone, Debug, PartialEq, Routable)]
pub enum Route {
    #[layout(AppLayout)]
        #[route("/")]
        Home {},
        #[route("/category/:category_id/:subcategory_id")]
        CategoryView { category_id: usize, subcategory_id: usize },
        #[route("/item/:id?:category_id&:subcategory_id")]
        ItemDetail { id: usize, category_id: usize, subcategory_id: usize },
        #[route("/universe/:id")]
        UniverseView { id: usize },
    #[end_layout]
    // Réglages du compte en plein écran, avec leur propre menu à gauche.
    #[route("/profile")]
    Profile {},
}