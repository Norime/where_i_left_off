//! Langue de l'interface. Chaque langue fournit une instance complète de
//! `Texts` (voir `fr.rs` / `en.rs`) : oublier un texte dans une langue est
//! une erreur de compilation, pas un trou à l'exécution.

mod en;
mod flags;
mod fr;

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::net::SearchError;
use crate::persistence::MediaError;
use crate::models::{ImageFit, ThemeColor, ThemeImageSlot, ThemePreset};
use crate::state::UserStore;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    pub account: &'static str,
    pub account_info: &'static str,
    pub appearance: &'static str,
    pub theme: &'static str,
    pub theme_hint: &'static str,
    pub theme_menu_background: &'static str,
    pub theme_menu_highlight: &'static str,
    pub theme_menu_text: &'static str,
    pub theme_category_text: &'static str,
    pub theme_category_background: &'static str,
    pub theme_subcategory_text: &'static str,
    pub theme_subcategory_background: &'static str,
    pub theme_primary: &'static str,
    pub theme_accent: &'static str,
    pub theme_success: &'static str,
    pub theme_danger: &'static str,
    pub theme_page_background: &'static str,
    pub theme_surface: &'static str,
    pub theme_text: &'static str,
    pub theme_presets: &'static str,
    pub theme_preset_default: &'static str,
    pub theme_preset_light: &'static str,
    pub theme_preset_forest: &'static str,
    pub theme_preset_dusk: &'static str,
    pub export_theme: &'static str,
    pub export_custom_only: &'static str,
    pub import_theme: &'static str,
    pub theme_exported: fn(&str) -> String,
    pub theme_export_failed: fn(&str) -> String,
    pub theme_import_ignored: fn(&str) -> String,
    pub theme_import_invalid: &'static str,
    pub theme_imported: fn(&str, usize, usize) -> String,
    pub custom_themes: &'static str,
    pub no_custom_themes: &'static str,
    pub create_theme: &'static str,
    pub theme_name: &'static str,
    pub new_theme_default_name: &'static str,
    pub theme_copy_name: fn(&str) -> String,
    pub edit_theme: fn(&str) -> String,
    pub delete_theme: &'static str,
    pub preset_not_editable: &'static str,
    pub imported_theme_default: &'static str,
    pub theme_missing_notice: &'static str,
    pub theme_image_missing_notice: &'static str,
    pub opacity: &'static str,
    pub colors_title: &'static str,
    pub colors_hint: &'static str,
    pub images_title: &'static str,
    pub images_hint: &'static str,
    pub image_slot_page_background: &'static str,
    pub image_slot_menu_background: &'static str,
    pub image_slot_category: &'static str,
    pub image_slot_subcategory: &'static str,
    pub remove_image: &'static str,
    pub image_fit: &'static str,
    pub image_fit_cover: &'static str,
    pub image_fit_contain: &'static str,
    pub image_fit_repeat: &'static str,
    pub image_opacity: &'static str,
    pub image_blur: &'static str,
    pub image_unsupported: &'static str,
    pub image_too_large: fn(&str) -> String,
    pub image_save_failed: fn(&str) -> String,
    pub from_file: &'static str,
    pub new_image: &'static str,
    pub profile_images: &'static str,
    pub profile_images_hint: &'static str,
    pub avatar_title: &'static str,
    pub login_background_title: &'static str,
    pub login_background_hint: &'static str,
    pub choose_profile: &'static str,
    pub no_background_hint: &'static str,
    pub crop_button: &'static str,
    pub crop_title: &'static str,
    pub crop_hint: &'static str,
    pub zoom: &'static str,
    pub recenter: &'static str,
    pub from_url: &'static str,
    pub download: &'static str,
    pub downloading: &'static str,
    pub image_url_invalid: &'static str,
    pub image_download_failed: fn(&str) -> String,
    pub image_download_http: fn(u16) -> String,
    pub export_title: fn(&str) -> String,
    pub export_include_images: fn(&str) -> String,
    pub export_images_hint: &'static str,
    pub export_no_images: &'static str,
    pub theme_import_images: fn(usize) -> String,
    pub theme_import_images_ignored: fn(usize) -> String,
    pub file_size: fn(u64) -> String,
    pub account_not_found: &'static str,
    pub has_password_hint: &'static str,
    pub no_password_hint: &'static str,
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
    pub settings: &'static str,
    pub category_settings_title: &'static str,
    pub subcategory_settings_title: &'static str,
    pub name_label: &'static str,
    pub translated_names: &'static str,
    pub translated_names_hint: &'static str,
    pub add_language: &'static str,
    pub home: &'static str,
    pub new_releases: &'static str,
    pub no_new_releases: &'static str,
    pub check_updates: &'static str,
    pub checking_updates: fn(usize, usize) -> String,
    pub new_count: fn(usize) -> String,
    pub mark_as_seen: &'static str,
    pub untracked_works: fn(usize) -> String,
    pub check_errors: &'static str,
    pub continue_tab: &'static str,
    pub no_continue: &'static str,
    pub next_up: fn(&str) -> String,
    pub progress_count: fn(usize, usize) -> String,
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
    pub test_query_placeholder: &'static str,
    pub testing: &'static str,
    pub run_test: &'static str,
    // --- Liste d'éléments importée par une source ---
    pub list_levels_hint: &'static str,
    pub add_list_level: &'static str,
    pub list_level_number: fn(usize) -> String,
    pub list_level_label_placeholder: &'static str,
    pub list_url_label: &'static str,
    pub list_items_path_label: &'static str,
    pub list_title_label: &'static str,
    pub list_image_label: &'static str,
    pub list_fields_title: &'static str,
    pub leaf_kind_label: &'static str,
    pub leaf_kind_text: &'static str,
    pub leaf_kind_card: &'static str,
    pub sublevel_hint: &'static str,
    pub pagination_label: &'static str,
    pub pagination_none: &'static str,
    pub pagination_page: &'static str,
    pub pagination_offset: &'static str,
    pub pagination_first_page: &'static str,
    pub pagination_page_size: &'static str,
    pub list_level_preview: fn(&str, usize) -> String,
    pub importing_list: &'static str,
    // --- Catalogue et éditeur de sources ---
    pub sources_tab: &'static str,
    pub catalog_intro: fn(&str) -> String,
    pub catalog_language: &'static str,
    pub catalog_reading: &'static str,
    pub catalog_movies: &'static str,
    pub catalog_series: &'static str,
    pub catalog_empty_kind: &'static str,
    pub catalog_api_key_needed: &'static str,
    pub catalog_tracks_list: &'static str,
    pub catalog_added_in: fn(&str) -> String,
    pub catalog_add: &'static str,
    pub catalog_linked_here: &'static str,
    pub add_catalog_title: fn(&str) -> String,
    pub add_catalog_where: &'static str,
    pub add_catalog_no_subcategory: &'static str,
    pub add_catalog_reuse_hint: &'static str,
    pub already_linked_here: &'static str,
    pub api_key_label: &'static str,
    pub api_key_hint: fn(&str) -> String,
    pub add_source_title: &'static str,
    pub from_catalog: &'static str,
    pub from_catalog_hint: &'static str,
    pub custom_source: &'static str,
    pub custom_source_hint: &'static str,
    pub reuse_existing: fn(usize) -> String,
    pub back_to_sources: &'static str,
    pub info_only: &'static str,
    pub step_where: &'static str,
    pub step_try: &'static str,
    pub step_extract: &'static str,
    pub step_list: &'static str,
    pub search_url_hint: &'static str,
    pub missing_query_marker: &'static str,
    pub test_found: fn(usize) -> String,
    pub test_no_results: &'static str,
    pub root_path: &'static str,
    pub path_hint: &'static str,
    pub preview_label: &'static str,
    pub run_test_first: &'static str,
    pub available_fields: &'static str,
    pub available_fields_hint: &'static str,
    pub use_as_title: &'static str,
    pub use_as_image: &'static str,
    pub use_as_description: &'static str,
    pub use_as_info: &'static str,
    pub advanced_options: &'static str,
    pub reimport_from: fn(&str) -> String,
    pub imported_entries: fn(usize) -> String,
    pub link_source: &'static str,
    pub link_source_hint: &'static str,
    pub new_badge: &'static str,
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
    /// Nom d'un thème prédéfini dans la langue courante.
    pub fn theme_preset_label(&self, preset: ThemePreset) -> &'static str {
        match preset {
            ThemePreset::Default => self.theme_preset_default,
            ThemePreset::Light => self.theme_preset_light,
            ThemePreset::Forest => self.theme_preset_forest,
            ThemePreset::Dusk => self.theme_preset_dusk,
        }
    }

    /// Message d'erreur quand une image (icône, image de thème) n'a pas pu
    /// être enregistrée.
    pub fn media_error(&self, error: &MediaError) -> String {
        match error {
            MediaError::Unsupported => self.image_unsupported.to_string(),
            MediaError::TooLarge => (self.image_too_large)(&(self.file_size)(crate::models::MAX_IMAGE_BYTES as u64)),
            MediaError::Io(detail) => (self.image_save_failed)(detail),
        }
    }

    /// Nom d'une zone d'image du thème dans la langue courante.
    pub fn image_slot_label(&self, slot: ThemeImageSlot) -> &'static str {
        match slot {
            ThemeImageSlot::PageBackground => self.image_slot_page_background,
            ThemeImageSlot::MenuBackground => self.image_slot_menu_background,
            ThemeImageSlot::Category => self.image_slot_category,
            ThemeImageSlot::Subcategory => self.image_slot_subcategory,
        }
    }

    /// Nom d'un mode d'affichage d'image dans la langue courante.
    pub fn image_fit_label(&self, fit: ImageFit) -> &'static str {
        match fit {
            ImageFit::Cover => self.image_fit_cover,
            ImageFit::Contain => self.image_fit_contain,
            ImageFit::Repeat => self.image_fit_repeat,
        }
    }

    /// Nom d'une couleur du thème dans la langue courante.
    pub fn theme_color_label(&self, color: ThemeColor) -> &'static str {
        match color {
            ThemeColor::MenuBackground => self.theme_menu_background,
            ThemeColor::MenuHighlight => self.theme_menu_highlight,
            ThemeColor::MenuText => self.theme_menu_text,
            ThemeColor::CategoryText => self.theme_category_text,
            ThemeColor::CategoryBackground => self.theme_category_background,
            ThemeColor::SubcategoryText => self.theme_subcategory_text,
            ThemeColor::SubcategoryBackground => self.theme_subcategory_background,
            ThemeColor::Primary => self.theme_primary,
            ThemeColor::Accent => self.theme_accent,
            ThemeColor::Success => self.theme_success,
            ThemeColor::Danger => self.theme_danger,
            ThemeColor::PageBackground => self.theme_page_background,
            ThemeColor::Surface => self.theme_surface,
            ThemeColor::Text => self.theme_text,
        }
    }

    /// Message d'erreur de recherche API dans la langue courante.
    pub fn search_error(&self, error: &SearchError) -> String {
        match error {
            SearchError::Unreachable(detail) => (self.api_unreachable)(detail),
            SearchError::HttpStatus(code) => (self.api_status)(code),
            SearchError::InvalidJson(detail) => (self.api_invalid_json)(detail),
        }
    }
}
