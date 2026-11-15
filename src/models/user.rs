use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
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
    /// Partie de l'icône affichée dans le cercle.
    #[serde(default)]
    pub avatar_crop: AvatarCrop,
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

/// Cadrage de l'icône dans son cercle : point de l'image à montrer (`x`,
/// `y` en %, 50/50 = centre) et zoom (1 = l'image remplit juste le cercle).
/// L'image n'est pas découpée : le cadrage est appliqué à l'affichage, ce
/// qui garde les GIF animés et permet de le refaire plus tard.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct AvatarCrop {
    pub x: f32,
    pub y: f32,
    pub zoom: f32,
}

impl AvatarCrop {
    pub const MAX_ZOOM: f32 = 4.0;

    /// Valeurs ramenées dans leurs limites (fichier modifié à la main…).
    pub fn clamped(self) -> AvatarCrop {
        let finite = |v: f32, default: f32| if v.is_finite() { v } else { default };
        AvatarCrop {
            x: finite(self.x, 50.0).clamp(0.0, 100.0),
            y: finite(self.y, 50.0).clamp(0.0, 100.0),
            zoom: finite(self.zoom, 1.0).clamp(1.0, Self::MAX_ZOOM),
        }
    }

    /// Style de l'image dans son cercle : `background-position` choisit la
    /// partie visible de l'image (rognée par `cover`), puis le zoom agrandit
    /// autour de ce même point. En pourcentages, le rendu est identique quelle
    /// que soit la taille du cercle (aperçu, écran de connexion, recadrage).
    pub fn style(&self, css_url: &str) -> String {
        let AvatarCrop { x, y, zoom } = self.clamped();
        format!(
            "background-image: {css_url}; background-position: {x}% {y}%; \
             transform: scale({zoom}); transform-origin: {x}% {y}%;"
        )
    }
}

impl Default for AvatarCrop {
    fn default() -> Self {
        AvatarCrop { x: 50.0, y: 50.0, zoom: 1.0 }
    }
}

/// Lecture tolérante : une valeur absente ou invalide prend sa valeur par
/// défaut au lieu d'empêcher le chargement des comptes.
impl<'de> Deserialize<'de> for AvatarCrop {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        let default = AvatarCrop::default();
        let field = |key: &str, fallback: f32| value.get(key).and_then(Value::as_f64).map(|v| v as f32).unwrap_or(fallback);
        Ok(AvatarCrop { x: field("x", default.x), y: field("y", default.y), zoom: field("zoom", default.zoom) }.clamped())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crop_is_read_leniently() {
        let crop: AvatarCrop = serde_json::from_str(r#"{ "x": 20, "zoom": 99 }"#).unwrap();
        assert_eq!(crop, AvatarCrop { x: 20.0, y: 50.0, zoom: AvatarCrop::MAX_ZOOM });
        let crop: AvatarCrop = serde_json::from_str("\"n'importe quoi\"").unwrap();
        assert_eq!(crop, AvatarCrop::default());
    }
}
