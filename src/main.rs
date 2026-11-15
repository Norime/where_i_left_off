use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use dioxus::prelude::*;

mod auth;
mod components;
mod i18n; // Language
mod json_path;
mod markdown;
mod models;
mod net;
mod pages;
mod persistence;
mod routes;
mod source_catalog;
mod xml_json;
mod source_import;
mod state;
mod update_check;

use models::{detect_image_type, ThemeImageSlot};
use components::{use_toast_provider, ToastHost};
use pages::AuthScreen;
use routes::Route;

const BASE_CSS: &str = include_str!("../assets/css/base.css");
const SIDEBAR_CSS: &str = include_str!("../assets/css/sidebar.css");
const ITEMS_CSS: &str = include_str!("../assets/css/items.css");
const MODAL_CSS: &str = include_str!("../assets/css/modal.css");
const AUTH_CSS: &str = include_str!("../assets/css/auth.css");
const PROFILE_CSS: &str = include_str!("../assets/css/profile.css");
const SOURCES_CSS: &str = include_str!("../assets/css/sources.css");
const LANGUAGE_CSS: &str = include_str!("../assets/css/language.css");
const HOME_CSS: &str = include_str!("../assets/css/home.css");

fn main() {
    dioxus::launch(App);
}

/// Point d'entrée de l'interface : charge la liste des comptes et l'état
/// de session, sauvegarde la liste des comptes à chaque changement, puis
/// affiche soit l'écran de connexion, soit l'application complète.
#[component]
fn App() -> Element {
    let mut user_store = use_context_provider(|| Signal::new(persistence::load_users()));
    let session = use_context_provider(|| Signal::new(None::<usize>));
    i18n::use_language_provider(user_store, session);

    use_effect(move || {
        persistence::save_users(&user_store.read());
    });

    let mut toasts = use_toast_provider();
    // Langue lue au moment d'afficher le message (elle change à la connexion).
    let language = use_context::<Memo<i18n::Language>>();

    // Thème choisi qui n'existe plus (prédéfini retiré de l'application,
    // thème perso absent du fichier) : retour au thème par défaut, et
    // message pour prévenir.
    use_effect(move || {
        let Some(user_id) = session() else { return };
        let missing = user_store.read().users.iter().any(|u| u.id == user_id && u.theme_missing());
        if missing && user_store.write().reset_missing_theme(user_id) {
            toasts.info(language.peek().texts().theme_missing_notice);
        }
    });

    // À la connexion : supprime les images de thème que plus aucun thème
    // du compte n'utilise (thème supprimé, image remplacée).
    use_effect(move || {
        let Some(user_id) = session() else { return };
        let used = user_store.peek().used_media(user_id);
        persistence::remove_unused_media(user_id, &used);
    });

    // Couleurs, règles des images et effet verre du compte connecté :
    // remplacent les valeurs par défaut de `base.css`. L'écran de connexion
    // garde l'apparence d'origine.
    let theme_css = session()
        .and_then(|id| user_store.read().users.iter().find(|u| u.id == id).map(|u| u.effective_theme().css()))
        .unwrap_or_default();

    // Contenu des images, dans sa propre feuille de style : les deux memos
    // évitent de relire et renvoyer les fichiers à chaque changement de
    // couleur ou de voile, seulement quand la liste des fichiers change.
    let image_files = use_memo(move || {
        let user_id = session()?;
        let store = user_store.read();
        let user = store.users.iter().find(|u| u.id == user_id)?;
        let files: Vec<(ThemeImageSlot, String)> =
            user.effective_theme().images().into_iter().map(|(slot, image)| (slot, image.file.clone())).collect();
        Some((user_id, files))
    });
    let image_css = use_memo(move || {
        let Some((user_id, files)) = image_files() else { return (String::new(), 0) };
        let mut declarations = String::new();
        let mut missing = 0;
        for (slot, file) in files {
            let data = persistence::load_media(user_id, &file)
                .and_then(|bytes| detect_image_type(&bytes).map(|(mime, _)| (mime, BASE64.encode(&bytes))));
            match data {
                Some((mime, encoded)) => {
                    declarations.push_str(&format!("{}: url(\"data:{mime};base64,{encoded}\"); ", slot.css_variable()))
                }
                None => missing += 1,
            }
        }
        (format!(":root {{ {declarations}}}"), missing)
    });
    // Image du thème introuvable (fichier supprimé du dossier) : la zone
    // garde sa couleur, et l'utilisateur est prévenu.
    use_effect(move || {
        if image_css().1 > 0 {
            toasts.info(language.peek().texts().theme_image_missing_notice);
        }
    });
    let image_data_css = image_css().0;

    // Mise en page du compte connecté : des classes sur l'enveloppe de
    // l'application, utilisées par les feuilles de style (position du
    // menu…).
    let layout_class = session()
        .and_then(|id| user_store.read().users.iter().find(|u| u.id == id).map(|u| u.layout.root_class()))
        .unwrap_or_default();

    let content = match session() {
        Some(user_id) => rsx! {
            div { class: "{layout_class}",
                AuthenticatedApp { user_id }
            }
        },
        None => rsx! {
            AuthScreen {}
        },
    };

    rsx! {
        style { {BASE_CSS} }
        style { {SIDEBAR_CSS} }
        style { {ITEMS_CSS} }
        style { {MODAL_CSS} }
        style { {AUTH_CSS} }
        style { {PROFILE_CSS} }
        style { {SOURCES_CSS} }
        style { {LANGUAGE_CSS} }
        style { {HOME_CSS} }
        style { {image_data_css} }
        style { {theme_css} }
        {content}
        ToastHost {}
    }
}

/// Partie de l'app affichée une fois connecté : charge les catégories, les
/// éléments, les types et les sources propres à `user_id`, les sauvegarde
/// automatiquement à chaque modification, puis monte le routeur (sidebar +
/// pages).
#[component]
fn AuthenticatedApp(user_id: usize) -> Element {
    let category_store = use_context_provider(|| Signal::new(persistence::load_categories(user_id)));
    let item_store = use_context_provider(|| Signal::new(persistence::load_items(user_id)));
    let type_store = use_context_provider(|| Signal::new(persistence::load_types(user_id)));
    let source_store = use_context_provider(|| Signal::new(persistence::load_sources(user_id)));
    update_check::use_update_checker(item_store, source_store);

    use_effect(move || {
        persistence::save_categories(user_id, &category_store.read());
    });

    use_effect(move || {
        persistence::save_items(user_id, &item_store.read());
    });

    use_effect(move || {
        persistence::save_types(user_id, &type_store.read());
    });

    use_effect(move || {
        persistence::save_sources(user_id, &source_store.read());
    });

    rsx! {
        Router::<Route> {}
    }
}