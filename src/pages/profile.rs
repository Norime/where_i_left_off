use dioxus::prelude::*;
use crate::components::LanguageSelect;
use crate::i18n::use_texts;
use crate::state::UserStore;

/// Page de réglages du compte connecté : modifier le nom d'utilisateur et
/// l'image de profil, choisir la langue, définir/changer/retirer le mot de
/// passe, et se déconnecter.
#[component]
pub fn Profile() -> Element {
    let i18n_text = use_texts();
    let mut user_store = use_context::<Signal<UserStore>>();
    let mut session = use_context::<Signal<Option<usize>>>();

    let Some(user_id) = session() else {
        return rsx! {
            p { "{i18n_text.no_user_logged_in}" }
        };
    };

    let current_user = user_store.read().users.iter().find(|u| u.id == user_id).cloned();

    let Some(current_user) = current_user else {
        return rsx! {
            p { "{i18n_text.account_not_found}" }
        };
    };

    let has_password = current_user.password_hash.is_some();
    let password_hint = if has_password {
        i18n_text.has_password_hint
    } else {
        i18n_text.no_password_hint
    };
    let current_language = current_user.language;

    let mut username = use_signal(|| current_user.username.clone());
    let mut avatar_url = use_signal(|| current_user.avatar_url.clone().unwrap_or_default());
    let mut profile_error = use_signal(|| None::<&'static str>);
    let mut profile_saved = use_signal(|| false);

    let mut new_password = use_signal(String::new);
    let mut password_error = use_signal(|| None::<&'static str>);
    let mut password_saved = use_signal(|| false);

    let avatar_url_value = avatar_url.read().trim().to_string();
    let avatar_preview_style = if avatar_url_value.is_empty() {
        String::new()
    } else {
        format!("background-image: url('{avatar_url_value}');")
    };

    rsx! {
        div { class: "profile-page",
            button {
                class: "back-button-inline",
                onclick: move |_| {
                    navigator().go_back();
                },
                "{i18n_text.back}"
            }
            h1 { "{i18n_text.profile_title}" }

            form {
                class: "profile-form",
                onsubmit: move |event| {
                    event.prevent_default();
                    let username_value = username.read().trim().to_string();
                    if username_value.is_empty() {
                        profile_error.set(Some(i18n_text.username_required));
                        profile_saved.set(false);
                        return;
                    }

                    let raw_avatar = avatar_url.read().trim().to_string();
                    let mut avatar = None;
                    if !raw_avatar.is_empty() {
                        avatar = Some(raw_avatar);
                    }

                    match user_store.write().update_user(user_id, username_value, avatar) {
                        Ok(()) => {
                            profile_error.set(None);
                            profile_saved.set(true);
                        }
                        Err(_) => {
                            profile_error.set(Some(i18n_text.username_taken));
                            profile_saved.set(false);
                        }
                    }
                },
                div { class: "profile-avatar-row",
                    div {
                        class: "profile-avatar-preview",
                        style: "{avatar_preview_style}",
                    }
                    label {
                        "{i18n_text.avatar_url_label}"
                        input {
                            value: "{avatar_url}",
                            oninput: move |event| {
                                avatar_url.set(event.value());
                                profile_saved.set(false);
                            },
                        }
                    }
                }
                label {
                    "{i18n_text.username}"
                    input {
                        value: "{username}",
                        oninput: move |event| {
                            username.set(event.value());
                            profile_saved.set(false);
                        },
                    }
                }
                if let Some(message) = profile_error() {
                    p { class: "auth-error", "{message}" }
                }
                if profile_saved() {
                    p { class: "profile-success", "{i18n_text.profile_saved}" }
                }
                button { r#type: "submit", class: "primary-btn", "{i18n_text.save}" }
            }

            // Appliquée immédiatement : le `Memo<Language>` de `App` lit
            // `user_store`, toute l'interface se ré-affiche dans la langue choisie.
            h2 { "{i18n_text.language}" }
            div { class: "form-field-language",
                LanguageSelect {
                    value: current_language,
                    onchange: move |l| user_store.write().set_language(user_id, l),
                }
                p { class: "profile-hint", "{i18n_text.language_hint}" }
            }

            h2 { "{i18n_text.password}" }
            p { class: "profile-hint", "{password_hint}" }
            form {
                class: "profile-form",
                onsubmit: move |event| {
                    event.prevent_default();
                    let pwd = new_password.read().trim().to_string();
                    if pwd.is_empty() {
                        password_error.set(Some(i18n_text.enter_password_first));
                        password_saved.set(false);
                        return;
                    }
                    user_store.write().set_password(user_id, pwd);
                    new_password.set(String::new());
                    password_error.set(None);
                    password_saved.set(true);
                },
                input {
                    r#type: "password",
                    placeholder: "{i18n_text.new_password_placeholder}",
                    value: "{new_password}",
                    oninput: move |event| {
                        new_password.set(event.value());
                        password_saved.set(false);
                    },
                }
                if let Some(message) = password_error() {
                    p { class: "auth-error", "{message}" }
                }
                if password_saved() {
                    p { class: "profile-success", "{i18n_text.password_saved}" }
                }
                div { class: "profile-password-actions",
                    button { r#type: "submit", class: "primary-btn", "{i18n_text.change_password}" }
                    if has_password {
                        button {
                            r#type: "button",
                            class: "danger-btn",
                            onclick: move |_| {
                                user_store.write().remove_password(user_id);
                                password_saved.set(true);
                            },
                            "{i18n_text.remove_password}"
                        }
                    }
                }
            }

            hr {}

            button { class: "danger-btn", onclick: move |_| session.set(None), "{i18n_text.logout}" }
        }
    }
}
