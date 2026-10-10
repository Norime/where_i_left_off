use dioxus::prelude::*;
use crate::components::NewAccountModal;
use crate::i18n::use_texts;

/// Cercle "+" en fin de rangée, qui ouvre la fenêtre de création de compte.
#[component]
pub fn AddAccountTile() -> Element {
    let i18n_text = use_texts();
    let mut show_modal = use_signal(|| false);

    rsx! {
        div { class: "account-tile",
            div {
                class: "account-avatar account-avatar--add",
                onclick: move |_| show_modal.set(true),
                span { class: "add-icon", "+" }
            }
            span { class: "account-name", "{i18n_text.new_account}" }
        }
        if show_modal() {
            NewAccountModal { onclose: move |_| show_modal.set(false) }
        }
    }
}