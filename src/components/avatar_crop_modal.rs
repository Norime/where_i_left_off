use dioxus::prelude::*;

use crate::components::{AvatarImage, Modal};
use crate::i18n::use_texts;
use crate::models::AvatarCrop;

/// Recadrage de l'icône : on fait glisser l'image dans le cercle pour
/// choisir la partie visible, et on règle le zoom. Rien n'est enregistré
/// avant « Enregistrer » (`onsave`).
#[component]
pub fn AvatarCropModal(css_url: String, initial: AvatarCrop, onsave: EventHandler<AvatarCrop>, onclose: EventHandler<()>) -> Element {
    let i18n_text = use_texts();
    let mut crop = use_signal(|| initial.clamped());
    // Glissement en cours : position de la souris et cadrage au départ.
    let mut drag = use_signal(|| None::<(f64, f64, AvatarCrop)>);
    // Recrée le curseur de zoom quand le cadrage est remis à zéro (il n'est
    // pas piloté par l'application, voir `SliderInput`).
    let mut zoom_version = use_signal(|| 0u32);
    let zoom_percent = (crop().zoom * 100.0).round() as i64;

    rsx! {
        Modal { onclose: move |_| onclose.call(()),
            h2 { "{i18n_text.crop_title}" }
            p { class: "profile-hint", "{i18n_text.crop_hint}" }
            div { class: "avatar-crop",
                div {
                    class: if drag().is_some() { "avatar-crop-stage avatar-crop-stage--dragging" } else { "avatar-crop-stage" },
                    onmousedown: move |e| {
                        e.prevent_default();
                        let point = e.client_coordinates();
                        drag.set(Some((point.x, point.y, crop())));
                    },
                    onmousemove: move |e| {
                        let Some((start_x, start_y, start)) = drag() else { return };
                        let point = e.client_coordinates();
                        // Faire glisser l'image vers la droite montre sa
                        // partie gauche : le point visé recule. Le zoom ralentit
                        // le déplacement pour garder la même sensation.
                        const STAGE_SIZE: f64 = 240.0;
                        let factor = 100.0 / STAGE_SIZE / start.zoom as f64;
                        crop.set(
                            AvatarCrop {
                                x: start.x - ((point.x - start_x) * factor) as f32,
                                y: start.y - ((point.y - start_y) * factor) as f32,
                                zoom: start.zoom,
                            }
                            .clamped(),
                        );
                    },
                    onmouseup: move |_| drag.set(None),
                    onmouseleave: move |_| drag.set(None),
                    AvatarImage { css_url: Some(css_url.clone()), crop: crop() }
                }
                label { class: "avatar-crop-zoom",
                    "{i18n_text.zoom}"
                    input {
                        key: "{zoom_version}",
                        r#type: "range",
                        min: 100,
                        max: (AvatarCrop::MAX_ZOOM * 100.0) as i64,
                        initial_value: "{zoom_percent}",
                        oninput: move |e| {
                            if let Ok(percent) = e.value().parse::<f32>() {
                                let current = crop();
                                crop.set(AvatarCrop { zoom: percent / 100.0, ..current }.clamped());
                            }
                        },
                    }
                    span { class: "theme-slider-unit", "{zoom_percent} %" }
                }
            }
            div { class: "modal-actions",
                button {
                    r#type: "button",
                    class: "theme-import-btn",
                    onclick: move |_| {
                        crop.set(AvatarCrop::default());
                        zoom_version += 1;
                    },
                    "{i18n_text.recenter}"
                }
                button { r#type: "button", onclick: move |_| onclose.call(()), "{i18n_text.cancel}" }
                button {
                    r#type: "button",
                    class: "primary-btn",
                    onclick: move |_| onsave.call(crop()),
                    "{i18n_text.save}"
                }
            }
        }
    }
}
