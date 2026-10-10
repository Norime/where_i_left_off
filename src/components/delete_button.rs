use dioxus::prelude::*;
use crate::components::Modal;
use crate::i18n::use_texts;

/// Bouton de suppression : ouvre une fenêtre de confirmation avant de
/// déclencher réellement `onconfirm`. Réutilisé pour les éléments, les
/// catégories, les sous-catégories et les comptes.
///
/// `prevent_default()` sur les 3 boutons est nécessaire quand ce composant
/// est placé à l'intérieur d'un `Link` (ex: sur les cartes d'items) : sans
/// ça, le clic finit par déclencher la navigation native du `<a>` du Link,
/// qui ouvre l'URL dans une fenêtre externe.
#[component]
pub fn DeleteButton(onconfirm: EventHandler<()>) -> Element {
    let i18n_text = use_texts();
    let mut show_confirm = use_signal(|| false);

    rsx! {
        button {
            class: "delete-btn",
            onclick: move |evt| {
                evt.stop_propagation();
                evt.prevent_default();
                show_confirm.set(true);
            },
            "🗑"
        }
        if show_confirm() {
            Modal { onclose: move |_| show_confirm.set(false),
                h2 { "{i18n_text.delete_confirm_title}" }
                p { "{i18n_text.delete_confirm_text}" }
                div { class: "modal-actions",
                    button {
                        r#type: "button",
                        onclick: move |evt| {
                            evt.stop_propagation();
                            evt.prevent_default();
                            show_confirm.set(false);
                        },
                        "{i18n_text.cancel}"
                    }
                    button {
                        r#type: "button",
                        class: "danger-btn",
                        onclick: move |evt| {
                            evt.stop_propagation();
                            evt.prevent_default();
                            show_confirm.set(false);
                            onconfirm.call(());
                        },
                        "{i18n_text.delete}"
                    }
                }
            }
        }
    }
}