use dioxus::prelude::*;

use crate::i18n::use_texts;

/// Écran d'accueil affiché tant qu'aucune catégorie n'est sélectionnée.
#[component]
pub fn Home() -> Element {
    let i18n_text = use_texts();
    rsx! {
        div { class: "empty-state",
            p { "{i18n_text.home_empty}" }
        }
    }
}