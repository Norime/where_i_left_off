use dioxus::prelude::*;

use crate::models::AvatarCrop;

/// Image d'une icône dans son cercle, avec son cadrage. À placer dans un
/// conteneur rond en `position: relative` (`.account-avatar`,
/// `.profile-avatar-preview`, zone de recadrage) : le rognage se fait dans
/// une couche à part, pour que ce qui dépasse du cercle (cadenas, bouton de
/// suppression) reste visible. Sans image, rien n'est affiché (le fond gris
/// du conteneur reste).
#[component]
pub fn AvatarImage(css_url: Option<String>, crop: AvatarCrop) -> Element {
    let Some(css_url) = css_url else { return rsx! {} };
    let style = crop.style(&css_url);
    rsx! {
        div { class: "avatar-clip",
            div { class: "avatar-image", style: "{style}" }
        }
    }
}
