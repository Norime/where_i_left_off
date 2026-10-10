//! Langue de l'interface. Chaque langue fournit une instance complète de
//! `Texts` (voir `fr.rs` / `en.rs`) : oublier un texte dans une langue est
//! une erreur de compilation, pas un trou à l'exécution.

mod en;
mod flags;
mod fr;

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::net::SearchError;
use crate::state::UserStore;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    Fr,
    En,
}

impl Language {
    /// Ordre d'affichage dans la liste déroulante.
    pub const ALL: [Language; 2] = [Language::Fr, Language::En];

    /// Nom de la langue écrit dans cette langue (« English » même quand
    /// l'interface est en français), pour qu'on la reconnaisse toujours.
    pub fn native_name(self) -> &'static str {
        match self {
            Language::Fr => "Français",
            Language::En => "English",
        }
    }

    /// Code ISO 639-1, servira aussi plus tard pour les API (ex: `?lang=fr`).
    pub fn code(self) -> &'static str {
        match self {
            Language::Fr => "fr",
            Language::En => "en",
        }
    }

    pub fn flag_svg(self) -> &'static str {
        match self {
            Language::Fr => flags::FR,
            Language::En => flags::GB,
        }
    }

    pub fn texts(self) -> &'static Texts {
        match self {
            Language::Fr => &fr::TEXTS,
            Language::En => &en::TEXTS,
        }
    }
}

/// Fournit la langue de l'interface à toute l'application : celle du compte
/// connecté, sinon celle mémorisée pour l'écran de connexion (dernier compte
/// connecté, ou choix fait avec le sélecteur en haut à droite). À appeler une
/// seule fois, dans `App`.
pub fn use_language_provider(mut user_store: Signal<UserStore>, session: Signal<Option<usize>>) {
    let language = use_memo(move || {
        let store = user_store.read();
        session()
            .and_then(|id| store.users.iter().find(|u| u.id == id))
            .map(|u| u.language)
            .unwrap_or(store.last_language)
    });
    use_context_provider(|| language);

    // Mémorise la langue du compte connecté pour le prochain écran de
    // connexion. `peek` évite que l'effet ne se relance sur sa propre écriture.
    use_effect(move || {
        let current = language();
        if session.peek().is_some() && user_store.peek().last_language != current {
            user_store.write().last_language = current;
        }
    });
}

/// Textes de la langue active. Lire le `Memo` fourni par
/// `use_language_provider` abonne le composant appelant : il se ré-affiche
/// dès que la langue change.
pub fn use_texts() -> &'static Texts {
    let language = use_context::<Memo<Language>>();
    language().texts()
}

/// Tous les textes de l'interface. Les champs `fn(...) -> String` sont les
/// textes avec une partie variable (appel : `(i18n_text.login_to)(&name)`).
pub struct Texts {
    // --- Commun ---
    pub cancel: &'static str,
    pub save: &'static str,
    pub create: &'static str,
    pub add: &'static str,
    pub close: &'static str,
    pub delete: &'static str,
    pub edit: &'static str,
    pub back: &'static str,
    pub title: &'static str,
    pub image_url: &'static str,
    pub description: &'static str,
    pub type_label: &'static str,
    pub format: &'static str,
    pub vertical: &'static str,
    pub horizontal: &'static str,
    pub loading: &'static str,
    pub nothing: &'static str,
    pub untitled: &'static str,
    pub watch: &'static str,
    pub language: &'static str,
    pub categories: &'static str,
    pub category: &'static str,
    pub subcategory: &'static str,
    pub other_categories: &'static str,
    pub no_type_for_category: &'static str,
    // --- Statuts ---
    pub status_label: &'static str,
    pub status_all: &'static str,
    pub status_new: &'static str,
    pub status_in_progress: &'static str,
    pub status_done: &'static str,
    pub work_status_label: &'static str,
    pub work_status_in_progress: &'static str,
    pub work_status_completed: &'static str,
    pub work_status_unfinished: &'static str,
    pub computed_from_progress: &'static str,
    // --- Comptes / connexion ---
    pub login_title: &'static str,
    pub no_account_hint: &'static str,
    pub new_account: &'static str,
    pub username: &'static str,
    pub password: &'static str,
    pub username_required: &'static str,
    pub username_taken: &'static str,
    pub protect_with_password: &'static str,
    pub empty_password_with_option: &'static str,
    pub login_to: fn(&str) -> String,
    pub wrong_password: &'static str,
    pub sign_in: &'static str,
    // --- Profil ---
    pub profile_title: &'static str,
    pub no_user_logged_in: &'static str,
    pub account_not_found: &'static str,
    pub has_password_hint: &'static str,
    pub no_password_hint: &'static str,
    pub avatar_url_label: &'static str,
    pub profile_saved: &'static str,
    pub new_password_placeholder: &'static str,
    pub enter_password_first: &'static str,
    pub password_saved: &'static str,
    pub change_password: &'static str,
    pub remove_password: &'static str,
    pub logout: &'static str,
    pub language_hint: &'static str,
    // --- Suppression ---
    pub delete_confirm_title: &'static str,
    pub delete_confirm_text: &'static str,
    // --- Sidebar / accueil ---
    pub new_category: &'static str,
    pub new_subcategory: &'static str,
    pub subcategory_all: &'static str,
    pub home_empty: &'static str,
    // --- Vue catégorie ---
    pub sort_title_asc: &'static str,
    pub sort_title_desc: &'static str,
    pub search_title_placeholder: &'static str,
    pub sources_for_subcategory: &'static str,
    pub sources_button: &'static str,
    pub select_subcategory_hint: &'static str,
    pub type_filter_all: &'static str,
    pub type_filter_selected: fn(usize) -> String,
    // --- Éléments ---
    pub new_item_title: &'static str,
    pub edit_item_title: &'static str,
    pub new_type: &'static str,
    pub add_to_category: &'static str,
    pub choose_category: &'static str,
    pub choose_subcategory: &'static str,
    pub choose_category_and_subcategory: &'static str,
    pub show_more: &'static str,
    pub show_less: &'static str,
    pub no_type: &'static str,
    pub item_not_found: &'static str,
    pub last_placement_error: &'static str,
    pub types_title: &'static str,
    pub new_type_for: fn(&str) -> String,
    pub this_category: &'static str,
    // --- Recherche via une source ---
    pub no_source_configured: &'static str,
    pub source: &'static str,
    pub search_label: &'static str,
    pub searching: &'static str,
    pub search_button: &'static str,
    pub api_unreachable: fn(&str) -> String,
    pub api_status: fn(&str) -> String,
    pub api_invalid_json: fn(&str) -> String,
    // --- Gestion des sources ---
    pub sources_intro: &'static str,
    pub linked_here: &'static str,
    pub no_linked_source: &'static str,
    pub new_source: &'static str,
    pub in_my_category: &'static str,
    pub no_other_source_in_category: &'static str,
    pub no_source_in_other_category: &'static str,
    pub used_in: fn(&str) -> String,
    pub link: &'static str,
    pub shared_source_warning: &'static str,
    pub source_name: &'static str,
    pub search_url_label: &'static str,
    pub results_path_label: &'static str,
    pub title_path_label: &'static str,
    pub image_path_label: &'static str,
    pub description_path_label: &'static str,
    pub extra_fields_title: &'static str,
    pub extra_field_label_placeholder: &'static str,
    pub json_path_placeholder: &'static str,
    pub add_field: &'static str,
    pub test_title: &'static str,
    pub test_hint: &'static str,
    pub test_query_placeholder: &'static str,
    pub testing: &'static str,
    pub run_test: &'static str,
    pub extracted_title: &'static str,
    pub extracted_image: &'static str,
    pub extracted_description: &'static str,
    // --- Onglets / progression ---
    pub progress: &'static str,
    pub progress_with_count: fn(usize, usize) -> String,
    pub remove_tab: &'static str,
    pub new_tab: &'static str,
    pub check_all: &'static str,
    pub edit_card: &'static str,
    pub inherited_links: &'static str,
    pub no_links: &'static str,
    pub link_name_placeholder: &'static str,
    pub duration_label: &'static str,
    pub progress_field_label_placeholder: &'static str,
    pub progress_field_value_placeholder: &'static str,
    pub new_card_title: &'static str,
    pub add_entry_placeholder: &'static str,
    pub list_name_placeholder: &'static str,
    pub sublist_name_placeholder: &'static str,
    pub contains_cards: &'static str,
    pub contains_text: &'static str,
    pub contains_sublists: &'static str,
}

impl Texts {
    /// Message d'erreur de recherche API dans la langue courante.
    pub fn search_error(&self, error: &SearchError) -> String {
        match error {
            SearchError::Unreachable(detail) => (self.api_unreachable)(detail),
            SearchError::HttpStatus(code) => (self.api_status)(code),
            SearchError::InvalidJson(detail) => (self.api_invalid_json)(detail),
        }
    }
}
