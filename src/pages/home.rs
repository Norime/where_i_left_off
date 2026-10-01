use dioxus::prelude::*;

/// Écran d'accueil affiché tant qu'aucune catégorie n'est sélectionnée.
#[component]
pub fn Home() -> Element {
    rsx! {
        div { class: "empty-state",
            p { "Sélectionne une catégorie dans le menu pour voir ses éléments." }
        }
    }
}