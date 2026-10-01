use dioxus::prelude::*;
use crate::components::DeleteButton;
use crate::models::User;

/// Un compte affiché en cercle : image (grise par défaut), nom en dessous,
/// cadenas jaune qui dépasse en bas si le compte est protégé, bouton de
/// suppression visible au survol. Cliquer sur le cercle ou le nom déclenche
/// `onselect` (au parent de décider : connexion directe ou demande de mot
/// de passe).
#[component]
pub fn AccountTile(user: User, onselect: EventHandler<()>, ondelete: EventHandler<()>) -> Element {
    let background_style = user.avatar_url
        .as_ref()
        .map(|url| format!("background-image: url('{url}');"))
        .unwrap_or_default();

    rsx! {
        div { class: "account-tile",
            div {
                class: "account-avatar",
                style: "{background_style}",
                onclick: move |_| onselect.call(()),
                if user.password_hash.is_some() {
                    span { class: "password-badge", "🔒" }
                }
                DeleteButton { onconfirm: move |_| ondelete.call(()) }
            }
            span { class: "account-name", onclick: move |_| onselect.call(()), "{user.username}" }
        }
    }
}