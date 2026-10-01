use dioxus::prelude::*;
use crate::auth::verify_password;
use crate::components::Modal;
use crate::state::UserStore;

/// Fenêtre de saisie du mot de passe pour se connecter à un compte protégé.
#[component]
pub fn PasswordPromptModal(user_id: usize, username: String, onclose: EventHandler<()>) -> Element {
    let user_store = use_context::<Signal<UserStore>>();
    let mut session = use_context::<Signal<Option<usize>>>();
    let mut password_input = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);

    rsx! {
        Modal { onclose: move |_| onclose.call(()),
            h2 { "Connexion à {username}" }
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
                        error.set(Some("Mot de passe incorrect".to_string()));
                    }
                },
                input {
                    r#type: "password",
                    placeholder: "Mot de passe",
                    value: "{password_input}",
                    oninput: move |event| password_input.set(event.value()),
                }
                if let Some(message) = error() {
                    p { class: "auth-error", "{message}" }
                }
                div { class: "modal-actions",
                    button { r#type: "button", onclick: move |_| onclose.call(()), "Annuler" }
                    button { r#type: "submit", class: "primary-btn", "Se connecter" }
                }
            }
        }
    }
}