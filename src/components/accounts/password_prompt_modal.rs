use dioxus::prelude::*;
use crate::auth::verify_password;
use crate::components::Modal;
use crate::i18n::use_texts;
use crate::state::UserStore;

/// Fenêtre de saisie du mot de passe pour se connecter à un compte protégé.
#[component]
pub fn PasswordPromptModal(user_id: usize, username: String, onclose: EventHandler<()>) -> Element {
    let i18n_text = use_texts();
    let user_store = use_context::<Signal<UserStore>>();
    let mut session = use_context::<Signal<Option<usize>>>();
    let mut password_input = use_signal(String::new);
    let mut error = use_signal(|| None::<&'static str>);

    rsx! {
        Modal { onclose: move |_| onclose.call(()),
            h2 { {(i18n_text.login_to)(&username)} }
            form {
                class: "new-item-modal-form",
                onsubmit: move |event| {
                    event.prevent_default();
                    let maybe_hash = user_store
                        .read()
                        .users
                        .iter()
                        .find(|u| u.id == user_id)
                        .and_then(|u| u.password_hash.clone());
                    let ok = match &maybe_hash {
                        Some(hash) => verify_password(&password_input.read(), hash),
                        None => true,
                    };
                    if ok {
                        session.set(Some(user_id));
                        onclose.call(());
                    } else {
                        error.set(Some(i18n_text.wrong_password));
                    }
                },
                input {
                    r#type: "password",
                    placeholder: "{i18n_text.password}",
                    value: "{password_input}",
                    oninput: move |event| password_input.set(event.value()),
                }
                if let Some(message) = error() {
                    p { class: "auth-error", "{message}" }
                }
                div { class: "modal-actions",
                    button { r#type: "button", onclick: move |_| onclose.call(()), "{i18n_text.cancel}" }
                    button { r#type: "submit", class: "primary-btn", "{i18n_text.sign_in}" }
                }
            }
        }
    }
}