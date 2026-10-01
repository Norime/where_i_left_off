use dioxus::prelude::*;
use crate::components::Modal;
use crate::state::UserStore;

/// Formulaire de création de compte : nom d'utilisateur et mot de passe
/// optionnel. Connecte automatiquement le nouveau compte une fois créé.
#[component]
pub fn NewAccountModal(onclose: EventHandler<()>) -> Element {
    let mut user_store = use_context::<Signal<UserStore>>();
    let mut session = use_context::<Signal<Option<usize>>>();

    let mut username = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut use_password = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    rsx! {
        Modal { onclose: move |_| onclose.call(()),
            h2 { "Nouveau compte" }
            form {
                class: "new-item-modal-form",
                onsubmit: move |event| {
                    event.prevent_default();
                    let username_value = username.read().trim().to_string();
                    if username_value.is_empty() {
                        error.set(Some("Le nom d'utilisateur est requis".to_string()));
                        return;
                    }

                    let password_value =
                    if use_password() {
                        let pwd = password.read().clone();
                        if pwd.is_empty() {
                            error
                                .set(
                                    Some(
                                        "Mot de passe vide alors que l'option est activée"
                                            .to_string(),
                                    ),
                                );
                            return;
                        }
                        Some(pwd)
                    } else {
                        None
                    };
                    match user_store.write().add_user(username_value, password_value) {
                        Ok(id) => {
                            error.set(None);
                            session.set(Some(id));
                            onclose.call(());
                        }
                        Err(message) => error.set(Some(message)),
                    }
                },
                label {
                    "Nom d'utilisateur"
                    input {
                        value: "{username}",
                        oninput: move |event| username.set(event.value()),
                    }
                }
                label {
                    input {
                        r#type: "checkbox",
                        checked: use_password(),
                        oninput: move |event| use_password.set(event.value().parse().unwrap_or(false)),
                    }
                    " Protéger avec un mot de passe"
                }
                if use_password() {
                    input {
                        r#type: "password",
                        placeholder: "Mot de passe",
                        value: "{password}",
                        oninput: move |event| password.set(event.value()),
                    }
                }
                if let Some(message) = error() {
                    p { class: "auth-error", "{message}" }
                }
                div { class: "modal-actions",
                    button { r#type: "button", onclick: move |_| onclose.call(()), "Annuler" }
                    button { r#type: "submit", class: "primary-btn", "Créer" }
                }
            }
        }
    }
}