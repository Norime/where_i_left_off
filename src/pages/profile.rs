use dioxus::prelude::*;
use crate::components::LanguageSelect;
use crate::i18n::use_texts;
use crate::models::User;
use crate::pages::theme_settings::ThemeSection;
use crate::state::UserStore;

/// Section affichée à droite du menu des réglages.
#[derive(Clone, Copy, PartialEq)]
enum ProfileSection {
    AccountInfo,
    Password,
    Theme,
    Icon,
}

/// Page de réglages du compte connecté, en plein écran : un menu à gauche
/// (groupes « Compte » et « Apparence », puis la déconnexion) et le contenu
/// de la section choisie à droite.
#[component]
pub fn Profile() -> Element {
    let i18n_text = use_texts();
    let user_store = use_context::<Signal<UserStore>>();
    let mut session = use_context::<Signal<Option<usize>>>();
    let mut section = use_signal(|| ProfileSection::AccountInfo);

    let current_user = session().and_then(|id| user_store.read().users.iter().find(|u| u.id == id).cloned());

    let menu_entry = move |target: ProfileSection, label: &'static str| {
        rsx! {
            button {
                r#type: "button",
                class: if section() == target { "profile-menu-entry active" } else { "profile-menu-entry" },
                onclick: move |_| section.set(target),
                "{label}"
            }
        }
    };

    rsx! {
        div { class: "profile-shell",
            nav { class: "profile-menu",
                button {
                    class: "profile-menu-back",
                    onclick: move |_| navigator().go_back(),
                    "{i18n_text.back}"
                }
                h2 { class: "profile-menu-title", "{i18n_text.account}" }
                {menu_entry(ProfileSection::AccountInfo, i18n_text.account_info)}
                {menu_entry(ProfileSection::Password, i18n_text.password)}
                h2 { class: "profile-menu-title", "{i18n_text.appearance}" }
                {menu_entry(ProfileSection::Theme, i18n_text.theme)}
                {menu_entry(ProfileSection::Icon, i18n_text.icon)}
                button {
                    class: "profile-menu-logout",
                    onclick: move |_| session.set(None),
                    "{i18n_text.logout}"
                }
            }
            main { class: "profile-page",
                match current_user {
                    None => rsx! {
                        p { "{i18n_text.account_not_found}" }
                    },
                    Some(user) => match section() {
                        ProfileSection::AccountInfo => rsx! {
                            AccountInfoSection { key: "{user.id}", user }
                        },
                        ProfileSection::Password => rsx! {
                            PasswordSection { key: "{user.id}", user }
                        },
                        ProfileSection::Theme => rsx! {
                            ThemeSection { key: "{user.id}", user }
                        },
                        ProfileSection::Icon => rsx! {
                            IconSection { key: "{user.id}", user }
                        },
                    },
                }
            }
        }
    }
}

/// Infos du compte : nom d'utilisateur et langue.
#[component]
fn AccountInfoSection(user: User) -> Element {
    let i18n_text = use_texts();
    let mut user_store = use_context::<Signal<UserStore>>();
    let user_id = user.id;

    let mut username = use_signal(|| user.username.clone());
    let mut profile_error = use_signal(|| None::<&'static str>);
    let mut profile_saved = use_signal(|| false);

    rsx! {
        h1 { "{i18n_text.account_info}" }
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

                match user_store.write().update_username(user_id, username_value) {
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
                value: user.language,
                onchange: move |l| user_store.write().set_language(user_id, l),
            }
            p { class: "profile-hint", "{i18n_text.language_hint}" }
        }
    }
}

/// Mot de passe : définir, changer ou retirer celui du compte.
#[component]
fn PasswordSection(user: User) -> Element {
    let i18n_text = use_texts();
    let mut user_store = use_context::<Signal<UserStore>>();
    let user_id = user.id;
    let has_password = user.password_hash.is_some();
    let password_hint = if has_password {
        i18n_text.has_password_hint
    } else {
        i18n_text.no_password_hint
    };

    let mut new_password = use_signal(String::new);
    let mut password_error = use_signal(|| None::<&'static str>);
    let mut password_saved = use_signal(|| false);

    rsx! {
        h1 { "{i18n_text.password}" }
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
    }
}

/// Icône : image de profil du compte (affichée sur l'écran de connexion).
#[component]
fn IconSection(user: User) -> Element {
    let i18n_text = use_texts();
    let mut user_store = use_context::<Signal<UserStore>>();
    let user_id = user.id;
    let has_avatar = user.avatar_url.is_some();

    let mut avatar_url = use_signal(|| user.avatar_url.clone().unwrap_or_default());
    let mut saved = use_signal(|| false);

    let avatar_url_value = avatar_url.read().trim().to_string();
    let avatar_preview_style = if avatar_url_value.is_empty() {
        String::new()
    } else {
        format!("background-image: url('{avatar_url_value}');")
    };

    rsx! {
        h1 { "{i18n_text.icon}" }
        p { class: "profile-hint", "{i18n_text.icon_hint}" }
        form {
            class: "profile-form",
            onsubmit: move |event| {
                event.prevent_default();
                let raw = avatar_url.read().trim().to_string();
                user_store.write().set_avatar(user_id, if raw.is_empty() { None } else { Some(raw) });
                saved.set(true);
            },
            div { class: "profile-avatar-row",
                div {
                    class: "profile-avatar-preview profile-avatar-preview--large",
                    style: "{avatar_preview_style}",
                }
                label {
                    "{i18n_text.avatar_url_label}"
                    input {
                        value: "{avatar_url}",
                        oninput: move |event| {
                            avatar_url.set(event.value());
                            saved.set(false);
                        },
                    }
                }
            }
            if saved() {
                p { class: "profile-success", "{i18n_text.profile_saved}" }
            }
            div { class: "profile-password-actions",
                button { r#type: "submit", class: "primary-btn", "{i18n_text.save}" }
                if has_avatar {
                    button {
                        r#type: "button",
                        class: "danger-btn",
                        onclick: move |_| {
                            user_store.write().set_avatar(user_id, None);
                            avatar_url.set(String::new());
                            saved.set(true);
                        },
                        "{i18n_text.remove_avatar}"
                    }
                }
            }
        }
    }
}
