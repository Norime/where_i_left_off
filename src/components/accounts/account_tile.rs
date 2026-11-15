use dioxus::prelude::*;
use crate::components::{AvatarImage, DeleteButton};
use crate::models::User;
use crate::persistence;

/// Un compte affiché en cercle : image (grise par défaut), nom en dessous,
/// cadenas jaune qui dépasse en bas si le compte est protégé, bouton de
/// suppression visible au survol. Cliquer sur le cercle ou le nom déclenche
/// `onselect` (au parent de décider : connexion directe ou demande de mot
/// de passe).
#[component]
pub fn AccountTile(user: User, onselect: EventHandler<()>, ondelete: EventHandler<()>) -> Element {
    // Recalculé seulement quand le compte change (lecture du fichier).
    let css_url = use_memo(use_reactive((&user,), |(user,)| persistence::avatar_css_url(&user)));
    let crop = user.avatar_crop;

    rsx! {
        div { class: "account-tile",
            div {
                class: "account-avatar",
                onclick: move |_| onselect.call(()),
                AvatarImage { css_url: css_url(), crop }
                if user.password_hash.is_some() {
                    span { class: "password-badge", "🔒" }
                }
                DeleteButton { onconfirm: move |_| ondelete.call(()) }
            }
            span { class: "account-name", onclick: move |_| onselect.call(()), "{user.username}" }
        }
    }
}