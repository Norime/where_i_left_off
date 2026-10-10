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
mod state;

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

fn main() {
    dioxus::launch(App);
}

/// Point d'entrée de l'interface : charge la liste des comptes et l'état
/// de session, sauvegarde la liste des comptes à chaque changement, puis
/// affiche soit l'écran de connexion, soit l'application complète.
#[component]
fn App() -> Element {
    let user_store = use_context_provider(|| Signal::new(persistence::load_users()));
    let session = use_context_provider(|| Signal::new(None::<usize>));
    i18n::use_language_provider(user_store, session);

    use_effect(move || {
        persistence::save_users(&user_store.read());
    });

    let content = match session() {
        Some(user_id) => rsx! {
            AuthenticatedApp { user_id }
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
        {content}
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