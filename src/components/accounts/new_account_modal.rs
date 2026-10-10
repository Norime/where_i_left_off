use dioxus::prelude::*;
use crate::components::{LanguageSelect, Modal};
use crate::i18n::{use_texts, Language};
use crate::state::UserStore;

/// Formulaire de création de compte : nom d'utilisateur, mot de passe
/// optionnel et langue. Connecte automatiquement le nouveau compte une fois créé.
#[component]
pub fn NewAccountModal(onclose: EventHandler<()>) -> Element {
    let i18n_text = use_texts();
    let mut user_store = use_context::<Signal<UserStore>>();
    let mut session = use_context::<Signal<Option<usize>>>();
    let current_language = use_context::<Memo<Language>>();

    let mut username = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut use_password = use_signal(|| false);
    // Par défaut : la langue affichée actuellement sur l'écran de connexion.
    let mut language = use_signal(|| current_language());
    let mut error = use_signal(|| None::<&'static str>);

    rsx! {
        Modal { onclose: move |_| onclose.call(()),
            h2 { "{i18n_text.new_account}" }
            form {
                class: "new-item-modal-form",
                onsubmit: move |event| {
                    event.prevent_default();
                    let username_value = username.read().trim().to_string();
                    if username_value.is_empty() {
                        error.set(Some(i18n_text.username_required));
                        return;
                    }

                    let password_value =
                    if use_password() {
                        let pwd = password.read().clone();
                        if pwd.is_empty() {
                            error.set(Some(i18n_text.empty_password_with_option));
                            return;
                        }
                        Some(pwd)
                    } else {
                        None
                    };
                    match user_store.write().add_user(username_value, password_value, language()) {
                        Ok(id) => {
                            error.set(None);
                            session.set(Some(id));
                            onclose.call(());
                        }
                        Err(_) => error.set(Some(i18n_text.username_taken)),
                    }
                },
                label {
                    "{i18n_text.username}"
                    input {
                        value: "{username}",
                        oninput: move |event| username.set(event.value()),
                    }
                }
                div { class: "form-field-language",
                    span { "{i18n_text.language}" }
                    LanguageSelect {
                        value: language(),
                        onchange: move |l| language.set(l),
                    }
                }
                label {
                    input {
                        r#type: "checkbox",
                        checked: use_password(),
                        oninput: move |event| use_password.set(event.value().parse().unwrap_or(false)),
                    }
                    " {i18n_text.protect_with_password}"
                }
                if use_password() {
                    input {
                        r#type: "password",
                        placeholder: "{i18n_text.password}",
                        value: "{password}",
                        oninput: move |event| password.set(event.value()),
                    }
                }
                if let Some(message) = error() {
                    p { class: "auth-error", "{message}" }
                }
                div { class: "modal-actions",
                    button { r#type: "button", onclick: move |_| onclose.call(()), "{i18n_text.cancel}" }
                    button { r#type: "submit", class: "primary-btn", "{i18n_text.create}" }
                }
            }
        }
    }
}
