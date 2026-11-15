use serde::{Deserialize, Serialize};
use crate::i18n::Language;
use crate::models::{deserialize_saved_themes, SavedTheme, Theme, ThemeChoice};

/// Un compte utilisateur local. `password_hash` vaut `None` si le compte
/// n'est pas protégé par mot de passe. `avatar_file` est l'image de profil
/// (réglages › Apparence › Icône), copiée dans le dossier du compte ;
/// `avatar_url` est l'ancienne icône par URL, gardée tant qu'elle n'est pas
/// remplacée. Le thème de l'interface est
/// `theme_choice`, qui désigne un prédéfini ou un des `saved_themes`
/// (thèmes créés ou importés par l'utilisateur).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub id: usize,
    pub username: String,
    pub password_hash: Option<String>,
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub avatar_file: Option<String>,
    #[serde(default)]
    pub language: Language,
    #[serde(default)]
    pub theme_choice: ThemeChoice,
    #[serde(default, deserialize_with = "deserialize_saved_themes")]
    pub saved_themes: Vec<SavedTheme>,
    /// Anciennes couleurs enregistrées directement dans le compte (avant
    /// les thèmes perso) : lues une fois puis converties par
    /// `UserStore::migrate`, jamais réécrites.
    #[serde(default, rename = "theme", skip_serializing)]
    pub legacy_theme: Option<Theme>,
}

impl User {
    /// Couleurs à appliquer : celles du thème choisi, ou celles par défaut
    /// s'il n'existe plus.
    pub fn effective_theme(&self) -> Theme {
        self.theme_choice.resolve(&self.saved_themes).unwrap_or_default()
    }

    /// Le thème choisi n'existe plus (prédéfini retiré, thème perso absent).
    pub fn theme_missing(&self) -> bool {
        self.theme_choice.resolve(&self.saved_themes).is_none()
    }

    /// Thème perso actuellement choisi, s'il y en a un.
    pub fn active_saved_theme(&self) -> Option<&SavedTheme> {
        match self.theme_choice {
            ThemeChoice::Custom(id) => self.saved_themes.iter().find(|t| t.id == id),
            ThemeChoice::Preset(_) => None,
        }
    }
}
