use dioxus::prelude::*;

use crate::i18n::use_texts;
use crate::models::{Layout, MenuPosition, User};
use crate::state::UserStore;

/// Réglages › Apparence › Mise en page : disposition de l'application,
/// propre au compte (indépendante du thème). Tout s'applique
/// immédiatement, la page des réglages sert d'aperçu.
#[component]
pub fn LayoutSection(user: User) -> Element {
    let i18n_text = use_texts();
    let mut user_store = use_context::<Signal<UserStore>>();
    let user_id = user.id;
    let layout = user.layout;

    rsx! {
        h1 { "{i18n_text.layout}" }
        p { class: "profile-hint", "{i18n_text.layout_hint}" }

        h2 { "{i18n_text.layout_menu_title}" }
        p { class: "profile-hint", "{i18n_text.layout_menu_hint}" }
        div { class: "theme-preset-list",
            for position in MenuPosition::ALL {
                button {
                    key: "{position.key()}",
                    r#type: "button",
                    class: if layout.menu_position == position { "theme-preset theme-preset--active" } else { "theme-preset" },
                    onclick: move |_| {
                        user_store.write().set_layout(user_id, Layout { menu_position: position, ..layout });
                    },
                    // Mini aperçu aux couleurs du thème : le menu du côté choisi.
                    div { class: "theme-preview layout-preview layout-preview--menu-{position.key()}",
                        div { class: "layout-preview-menu" }
                        div { class: "layout-preview-page" }
                    }
                    span { class: "theme-preset-name", "{i18n_text.menu_position_label(position)}" }
                }
            }
        }
    }
}
