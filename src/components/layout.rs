use dioxus::prelude::*;
use crate::routes::Route;
use crate::components::Sidebar;

/// Ossature visuelle commune à toutes les pages une fois connecté : la
/// sidebar reste affichée en permanence, seul le contenu de l'Outlet
/// change selon la route active.
#[component]
pub fn AppLayout() -> Element {
    rsx! {
        div { class: "app-shell",
            Sidebar {}
            main { class: "main-content", Outlet::<Route> {} }
        }
    }
}