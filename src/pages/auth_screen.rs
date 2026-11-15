use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use dioxus::prelude::*;
use crate::components::{AccountTile, AddAccountTile, LanguageSelect, PasswordPromptModal};
use crate::i18n::use_texts;
use crate::persistence;
use crate::state::UserStore;

/// Calques gardés pour le fondu entre les fonds des profils.
const MAX_BACKGROUND_LAYERS: usize = 3;

/// Un fond de l'écran de connexion, en cours d'apparition ou affiché.
#[derive(Clone, PartialEq)]
struct BackgroundLayer {
    id: u64,
    /// `None` : profil sans fond (dégradé neutre).
    css_url: Option<String>,
}

/// Écran affiché tant que personne n'est connecté, sur le principe d'un
/// écran de choix de personnage : le fond du profil survolé occupe tout
/// l'écran, les profils sont alignés un peu sous le milieu avec une tuile « + » pour
/// en créer un. Un clic sur un profil connecte, avec une demande de mot de
/// passe si besoin.
#[component]
pub fn AuthScreen() -> Element {
    let i18n_text = use_texts();
    let mut user_store = use_context::<Signal<UserStore>>();
    let mut session = use_context::<Signal<Option<usize>>>();
    let mut password_prompt = use_signal(|| None::<(usize, String)>);
    // Dernier profil survolé : le fond reste le sien quand la souris quitte
    // la rangée.
    let mut hovered = use_signal(|| None::<usize>);
    // Fonds déjà lus, par profil et fichier : survoler les profils ne relit
    // pas les images à chaque fois. Pas un signal : rien à ré-afficher.
    let backgrounds = use_hook(|| Rc::new(RefCell::new(HashMap::<(usize, Option<String>), Option<String>>::new())));
    let last_language = user_store.read().last_language;

    let users = user_store.read().users.clone();
    // Avant tout survol (ou si le profil survolé a été supprimé) : le premier.
    let preview = hovered()
        .and_then(|id| users.iter().find(|u| u.id == id))
        .or(users.first())
        .cloned();
    let preview_id = preview.as_ref().map(|u| u.id);
    let background = preview.as_ref().and_then(|user| {
        backgrounds
            .borrow_mut()
            .entry((user.id, user.login_background.clone()))
            .or_insert_with(|| persistence::login_background_css_url(user))
            .clone()
    });
    // Fondu enchaîné : chaque nouveau fond est un calque posé par-dessus
    // les précédents, qui apparaît en fondu (animation CSS au montage).
    // Seuls les derniers calques sont gardés : un survol rapide de plusieurs
    // profils n'en accumule pas. Change seulement quand le profil ou son
    // fichier de fond change (clé légère, pas l'image elle-même).
    let mut layers = use_signal(Vec::<BackgroundLayer>::new);
    let mut next_layer_id = use_signal(|| 0u64);
    let layer_key = preview.as_ref().map(|u| (u.id, u.login_background.clone()));
    // `use_effect` garde la closure du premier rendu : le fond est donc
    // retrouvé à partir de la clé, dans le cache rempli juste au-dessus par
    // le rendu en cours.
    let cache = backgrounds.clone();
    use_effect(use_reactive((&layer_key,), move |(key,)| {
        let css_url = key.and_then(|key| cache.borrow().get(&key).cloned().flatten());
        let id = *next_layer_id.peek();
        next_layer_id.set(id + 1);
        let mut list = layers.write();
        list.push(BackgroundLayer { id, css_url });
        let excess = list.len().saturating_sub(MAX_BACKGROUND_LAYERS);
        list.drain(..excess);
    }));

    let mut connect = move |user_id: usize| {
        let store = user_store.read();
        let Some(user) = store.users.iter().find(|u| u.id == user_id) else { return };
        if user.password_hash.is_some() {
            password_prompt.set(Some((user_id, user.username.clone())));
        } else {
            drop(store);
            session.set(Some(user_id));
        }
    };

    rsx! {
        div { class: "login-screen",
            for layer in layers() {
                div {
                    key: "{layer.id}",
                    class: if layer.css_url.is_some() { "login-background" } else { "login-background login-background--empty" },
                    style: if let Some(url) = &layer.css_url { "background-image: {url};" },
                }
            }
            div { class: "login-shade" }

            div { class: "auth-language-switch",
                LanguageSelect {
                    value: last_language,
                    compact: true,
                    onchange: move |l| user_store.write().last_language = l,
                }
            }
            h1 { class: "login-title", "{i18n_text.choose_profile}" }

            // Un peu sous le milieu de l'écran (62,5 % de la hauteur) : nom du
            // profil survolé, puis la rangée de profils.
            div { class: "login-center",
                match preview {
                    Some(user) => rsx! {
                        div { class: "login-focus",
                            span { class: "login-focus-name",
                                "{user.username}"
                                if user.password_hash.is_some() {
                                    span { class: "login-focus-lock", "🔒" }
                                }
                            }
                        }
                    },
                    None => rsx! {
                        p { class: "login-empty", "{i18n_text.no_account_hint}" }
                    },
                }

                div { class: "profile-row",
                    for user in users.iter().cloned() {
                        AccountTile {
                            key: "{user.id}",
                            highlighted: preview_id == Some(user.id),
                            onhover: move |id| hovered.set(Some(id)),
                            onselect: move |id| connect(id),
                            ondelete: move |id: usize| {
                                user_store.write().remove_user(id);
                                persistence::delete_user_data(id);
                                if password_prompt().map(|(prompt_id, _)| prompt_id) == Some(id) {
                                    password_prompt.set(None);
                                }
                                if hovered() == Some(id) {
                                    hovered.set(None);
                                }
                            },
                            user,
                        }
                    }
                    AddAccountTile {}
                }
                // Place toujours réservée (vide si le profil a un fond) : le
                // message n'apparaît pas en décalant les profils.
                span { class: "login-focus-hint",
                    if preview_id.is_some() && background.is_none() { "{i18n_text.no_background_hint}" }
                }
            }
        }

        if let Some((user_id, username)) = password_prompt() {
            PasswordPromptModal {
                user_id,
                username,
                onclose: move |_| password_prompt.set(None),
            }
        }
    }
}
