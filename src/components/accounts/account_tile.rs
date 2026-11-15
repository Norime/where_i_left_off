use dioxus::prelude::*;
use crate::components::{AvatarImage, DeleteButton};
use crate::models::User;
use crate::persistence;

/// Un profil de l'écran de connexion : portrait carré (icône cadrée, gris
/// par défaut), nom en dessous, cadenas si le compte est protégé, bouton
/// de suppression au survol. Survol → `onhover` (le fond change), clic →
/// `onselect` (connexion). `highlighted` : profil dont le fond est affiché.
#[component]
pub fn AccountTile(
    user: User,
    highlighted: bool,
    onhover: EventHandler<usize>,
    onselect: EventHandler<usize>,
    ondelete: EventHandler<usize>,
) -> Element {
    let user_id = user.id;
    // Recalculé seulement quand le compte change (lecture du fichier).
    let css_url = use_memo(use_reactive((&user,), |(user,)| persistence::avatar_css_url(&user)));
    let crop = user.avatar_crop;

    rsx! {
        div {
            class: if highlighted { "profile-tile profile-tile--selected" } else { "profile-tile" },
            title: "{user.username}",
            onmouseenter: move |_| onhover.call(user_id),
            onclick: move |_| onselect.call(user_id),
            div { class: "profile-portrait",
                AvatarImage { css_url: css_url(), crop }
                if user.password_hash.is_some() {
                    span { class: "password-badge", "🔒" }
                }
                DeleteButton { onconfirm: move |_| ondelete.call(user_id) }
            }
            span { class: "profile-tile-name", "{user.username}" }
        }
    }
}
