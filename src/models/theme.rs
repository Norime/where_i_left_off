use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

use crate::models::theme_image::{
    detect_image_type, fit_from_value, opacity_from_value, ImageFit, ThemeImage, ThemeImageSlot, MAX_IMAGE_BYTES,
};

/// Une couleur modifiable du thème. Pour en ajouter une : un variant, une
/// ligne dans `ALL`, `key`, `css_variable`, `default_value`, son libellé
/// (`Texts::theme_color_label`) et éventuellement sa valeur dans les thèmes
/// prédéfinis (sinon ils prennent la valeur par défaut). En retirer une ne
/// casse rien : sa clé est simplement ignorée dans les fichiers existants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeColor {
    MenuBackground,
    MenuHighlight,
    MenuText,
    CategoryText,
    CategoryBackground,
    SubcategoryText,
    SubcategoryBackground,
    Primary,
    Accent,
    Success,
    Danger,
    PageBackground,
    Surface,
    Text,
}

impl ThemeColor {
    /// Ordre d'affichage dans les réglages.
    pub const ALL: [ThemeColor; 14] = [
        ThemeColor::MenuBackground,
        ThemeColor::MenuHighlight,
        ThemeColor::MenuText,
        ThemeColor::CategoryText,
        ThemeColor::CategoryBackground,
        ThemeColor::SubcategoryText,
        ThemeColor::SubcategoryBackground,
        ThemeColor::Primary,
        ThemeColor::Accent,
        ThemeColor::Success,
        ThemeColor::Danger,
        ThemeColor::PageBackground,
        ThemeColor::Surface,
        ThemeColor::Text,
    ];

    /// Clé stable dans les fichiers (`users.json` et thèmes exportés) : ne
    /// pas la renommer, sinon les anciens fichiers perdent cette couleur.
    pub fn key(self) -> &'static str {
        match self {
            ThemeColor::MenuBackground => "menu_background",
            ThemeColor::MenuHighlight => "menu_highlight",
            ThemeColor::MenuText => "menu_text",
            ThemeColor::CategoryText => "category_text",
            ThemeColor::CategoryBackground => "category_background",
            ThemeColor::SubcategoryText => "subcategory_text",
            ThemeColor::SubcategoryBackground => "subcategory_background",
            ThemeColor::Primary => "primary",
            ThemeColor::Accent => "accent",
            ThemeColor::Success => "success",
            ThemeColor::Danger => "danger",
            ThemeColor::PageBackground => "page_background",
            ThemeColor::Surface => "surface",
            ThemeColor::Text => "text",
        }
    }

    /// Nom de la variable CSS correspondante (voir `assets/css/base.css`).
    pub fn css_variable(self) -> &'static str {
        match self {
            ThemeColor::MenuBackground => "--menu-bg",
            ThemeColor::MenuHighlight => "--menu-highlight",
            ThemeColor::MenuText => "--menu-text",
            ThemeColor::CategoryText => "--category-text",
            ThemeColor::CategoryBackground => "--category-bg",
            ThemeColor::SubcategoryText => "--subcategory-text",
            ThemeColor::SubcategoryBackground => "--subcategory-bg",
            ThemeColor::Primary => "--primary",
            ThemeColor::Accent => "--accent",
            ThemeColor::Success => "--success",
            ThemeColor::Danger => "--danger",
            ThemeColor::PageBackground => "--page-bg",
            ThemeColor::Surface => "--surface",
            ThemeColor::Text => "--text",
        }
    }

    /// Couleur d'origine de l'application. Pour les textes des catégories
    /// et sous-catégories, ce n'est qu'un repli : tant qu'ils ne sont pas
    /// choisis, ils suivent le texte du menu (voir `Theme::get`).
    pub fn default_value(self) -> &'static str {
        match self {
            ThemeColor::MenuBackground => "#1e1e2f",
            ThemeColor::MenuHighlight => "#3a3a5c",
            ThemeColor::MenuText => "#ffffff",
            ThemeColor::CategoryText => "#ffffff",
            ThemeColor::CategoryBackground => "#00000000",
            ThemeColor::SubcategoryText => "#ffffffcc",
            ThemeColor::SubcategoryBackground => "#00000000",
            ThemeColor::Primary => "#1e1e2f",
            ThemeColor::Accent => "#2a8f88",
            ThemeColor::Success => "#2e8b57",
            ThemeColor::Danger => "#c0392b",
            ThemeColor::PageBackground => "#ffffff",
            ThemeColor::Surface => "#ffffff",
            ThemeColor::Text => "#222222",
        }
    }

    fn from_key(key: &str) -> Option<ThemeColor> {
        ThemeColor::ALL.into_iter().find(|color| color.key() == key)
    }
}

/// Apparence choisie par un compte : couleurs (`#rrggbb` ou `#rrggbbaa`
/// avec transparence) rangées par clé, images de fond, et flou « verre »
/// derrière les cartes. Une couleur absente prend sa valeur par défaut : un
/// thème enregistré avant l'ajout d'une couleur reste donc valide.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Theme {
    colors: BTreeMap<String, String>,
    images: BTreeMap<String, ThemeImage>,
    glass_blur: u8,
}

/// Chargement tolérant depuis `users.json` : une valeur invalide ou une clé
/// inconnue est ignorée au lieu d'empêcher la lecture de tous les comptes.
/// Accepte aussi l'ancien format (un simple objet de couleurs).
impl<'de> Deserialize<'de> for Theme {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        let mut theme = Theme::from_colors_value(colors_object(&value).unwrap_or(&Value::Null)).0;
        theme.glass_blur = glass_blur_from_value(&value);
        if let Some(images) = value.get("images").and_then(Value::as_object) {
            for (key, raw) in images {
                if let (Some(slot), Some(image)) = (ThemeImageSlot::from_key(key), ThemeImage::from_value(raw)) {
                    theme.images.insert(slot.key().to_string(), image);
                }
            }
        }
        Ok(theme)
    }
}

/// Bilan d'un import, affiché à l'utilisateur.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ImportReport {
    /// Couleurs reprises du fichier.
    pub applied: usize,
    /// Couleurs que le fichier ne contenait pas (valeur par défaut gardée).
    pub missing: usize,
    /// Clés inconnues (couleur retirée de l'application) ou valeurs invalides.
    pub ignored: Vec<String>,
    /// Images reprises du fichier.
    pub images_applied: usize,
    /// Images inconnues, abîmées, trop lourdes ou d'un format non pris en charge.
    pub images_ignored: Vec<String>,
}

/// Image contenue dans un fichier de thème importé, à enregistrer dans le
/// dossier du compte avant de l'ajouter au thème (voir
/// `persistence::save_imported_images`).
#[derive(Clone, Debug, PartialEq)]
pub struct EmbeddedImage {
    pub slot: ThemeImageSlot,
    pub fit: ImageFit,
    pub opacity: u8,
    pub bytes: Vec<u8>,
}

/// Résultat d'un import réussi. `name` vaut `None` si le fichier n'en a pas.
#[derive(Clone, Debug, PartialEq)]
pub struct ImportedTheme {
    pub name: Option<String>,
    pub theme: Theme,
    pub images: Vec<EmbeddedImage>,
    pub report: ImportReport,
}

/// Le fichier n'est pas du JSON, ou ne contient aucun objet de couleurs.
#[derive(Debug)]
pub struct InvalidThemeFile;

const EXPORT_FORMAT: &str = "where_i_left_off-theme";
/// Flou maximal de l'effet verre, en pixels.
pub const MAX_GLASS_BLUR: u8 = 20;

/// Les couleurs d'un thème : l'objet `colors`, ou le thème entier pour
/// l'ancien format à plat.
fn colors_object(value: &Value) -> Option<&Value> {
    match value.get("colors") {
        Some(colors) if colors.is_object() => Some(colors),
        _ if value.is_object() => Some(value),
        _ => None,
    }
}

fn glass_blur_from_value(value: &Value) -> u8 {
    value.get("glass_blur").and_then(Value::as_u64).map(|v| v.min(MAX_GLASS_BLUR as u64) as u8).unwrap_or(0)
}

impl Theme {
    /// Valeur effective d'une couleur : celle choisie, sinon sa valeur par
    /// défaut. Les textes des catégories et sous-catégories non choisis
    /// suivent le texte du menu (atténué pour les sous-catégories), pour que
    /// les thèmes d'avant leur ajout gardent leur rendu.
    pub fn get(&self, color: ThemeColor) -> String {
        if let Some(value) = self.colors.get(color.key()) {
            return value.clone();
        }
        match color {
            ThemeColor::CategoryText => self.get(ThemeColor::MenuText),
            ThemeColor::SubcategoryText => {
                let (rgb, opacity) = split_alpha(&self.get(ThemeColor::MenuText));
                with_alpha(&rgb, (opacity as u16 * 80 / 100) as u8)
            }
            _ => color.default_value().to_string(),
        }
    }

    /// Change une couleur, seulement si `value` est une couleur valide
    /// (une valeur invalide casserait toute la feuille de style).
    pub fn set(&mut self, color: ThemeColor, value: &str) {
        if let Some(value) = normalize_hex(value) {
            self.colors.insert(color.key().to_string(), value);
        }
    }

    pub fn image(&self, slot: ThemeImageSlot) -> Option<&ThemeImage> {
        self.images.get(slot.key())
    }

    /// Ajoute, remplace ou retire (`None`) l'image d'une zone.
    pub fn set_image(&mut self, slot: ThemeImageSlot, image: Option<ThemeImage>) {
        match image {
            Some(image) => self.images.insert(slot.key().to_string(), image),
            None => self.images.remove(slot.key()),
        };
    }

    /// Images présentes, dans l'ordre de `ThemeImageSlot::ALL`.
    pub fn images(&self) -> Vec<(ThemeImageSlot, &ThemeImage)> {
        ThemeImageSlot::ALL.iter().filter_map(|slot| self.image(*slot).map(|image| (*slot, image))).collect()
    }

    pub fn glass_blur(&self) -> u8 {
        self.glass_blur
    }

    pub fn set_glass_blur(&mut self, blur: u8) {
        self.glass_blur = blur.min(MAX_GLASS_BLUR);
    }

    /// Construit un thème à partir de quelques couleurs, les autres gardant
    /// leur valeur par défaut (sert aux thèmes prédéfinis).
    fn from_pairs(pairs: &[(ThemeColor, &str)]) -> Theme {
        let mut theme = Theme::default();
        for (color, value) in pairs {
            theme.set(*color, value);
        }
        theme
    }

    /// Même rendu (toutes les couleurs effectives identiques) ?
    pub fn same_colors(&self, other: &Theme) -> bool {
        ThemeColor::ALL.iter().all(|color| self.get(*color) == other.get(*color))
    }

    /// Feuille de style du thème, sauf le contenu des images (voir
    /// `ThemeImageSlot::css_variable`) : variables de couleur, règles des
    /// images de fond et effet verre.
    pub fn css(&self) -> String {
        let declarations: String = ThemeColor::ALL
            .iter()
            .map(|color| format!("{}: {}; ", color.css_variable(), self.get(*color)))
            .collect();
        let mut css = format!(":root {{ {declarations}}}\n");
        for (slot, image) in self.images() {
            css.push_str(&image.css_rule(slot));
            css.push('\n');
        }
        if self.glass_blur > 0 {
            let blur = format!("backdrop-filter: blur({0}px); -webkit-backdrop-filter: blur({0}px);", self.glass_blur);
            // Menu : le flou est posé sur un calque fixe derrière lui (même
            // place, 220 px de large) et pas sur le menu lui-même, sinon
            // `backdrop-filter` enfermerait dans le menu les fenêtres qui s'y
            // ouvrent (configuration d'une catégorie). Le calque est peint
            // après le fond des pages et avant le fond du menu : un menu
            // transparent laisse voir le fond des pages flouté.
            css.push_str(&format!(
                ".sidebar::before, .profile-menu::before {{ content: \"\"; position: fixed; top: 0; bottom: 0; left: 0; \
                 width: 220px; z-index: -1; pointer-events: none; {blur} }}\n"
            ));
            // Cartes du contenu. Pas sur les fenêtres modales, pour la même
            // raison (la confirmation de suppression s'ouvre dans une modale).
            css.push_str(&format!(
                ".home-release, .catalog-card, .source-add-choice, .editor-step, .source-result-item, .app-notice {{ {blur} }}\n"
            ));
        }
        css
    }

    /// Fichier de thème à partager : un nom, un numéro de version du format,
    /// toutes les couleurs actuelles (y compris celles par défaut, pour que
    /// le fichier reste identique même si les défauts changent), l'effet
    /// verre, et les images fournies dans `image_bytes` (clé de zone →
    /// contenu), encodées en base64. Une image sans contenu n'est pas
    /// exportée : passer une map vide exporte le thème sans images.
    pub fn export_json(&self, name: &str, image_bytes: &BTreeMap<String, Vec<u8>>) -> String {
        let colors: Map<String, Value> = ThemeColor::ALL
            .iter()
            .map(|color| (color.key().to_string(), Value::String(self.get(*color))))
            .collect();
        let images: Map<String, Value> = self
            .images()
            .into_iter()
            .filter_map(|(slot, image)| {
                let bytes = image_bytes.get(slot.key())?;
                let (mime, _) = detect_image_type(bytes)?;
                Some((
                    slot.key().to_string(),
                    serde_json::json!({
                        "fit": image.fit.key(),
                        "opacity": image.opacity,
                        "mime": mime,
                        "data": BASE64.encode(bytes),
                    }),
                ))
            })
            .collect();
        let mut file = serde_json::json!({
            "format": EXPORT_FORMAT,
            "version": 2,
            "name": name,
            "colors": colors,
            "glass_blur": self.glass_blur,
        });
        if !images.is_empty() {
            file["images"] = Value::Object(images);
        }
        serde_json::to_string_pretty(&file).unwrap_or_default()
    }

    /// Lit un fichier de thème, même ancien ou modifié à la main : accepte
    /// `{ "colors": { … } }` comme un simple objet de couleurs, reprend
    /// chaque couleur et chaque image connue et valide, et ignore le reste.
    pub fn import_json(text: &str) -> Result<ImportedTheme, InvalidThemeFile> {
        let value: Value = serde_json::from_str(text).map_err(|_| InvalidThemeFile)?;
        let name = value
            .get("name")
            .and_then(Value::as_str)
            .map(|name| name.trim().to_string())
            .filter(|name| !name.is_empty());
        let colors = colors_object(&value).ok_or(InvalidThemeFile)?;
        let (mut theme, mut report) = Theme::from_colors_value(colors);
        // Pas une seule couleur reconnue : ce n'est pas un fichier de thème.
        if report.applied == 0 {
            return Err(InvalidThemeFile);
        }
        theme.glass_blur = glass_blur_from_value(&value);

        let mut images = Vec::new();
        if let Some(entries) = value.get("images").and_then(Value::as_object) {
            for (key, raw) in entries {
                let bytes = raw
                    .get("data")
                    .and_then(Value::as_str)
                    .and_then(|data| BASE64.decode(data.trim()).ok())
                    .filter(|bytes| bytes.len() <= MAX_IMAGE_BYTES && detect_image_type(bytes).is_some());
                match (ThemeImageSlot::from_key(key), bytes) {
                    (Some(slot), Some(bytes)) => {
                        images.push(EmbeddedImage { slot, fit: fit_from_value(raw), opacity: opacity_from_value(raw), bytes });
                        report.images_applied += 1;
                    }
                    _ => report.images_ignored.push(key.clone()),
                }
            }
        }
        Ok(ImportedTheme { name, theme, images, report })
    }

    fn from_colors_value(value: &Value) -> (Theme, ImportReport) {
        let mut theme = Theme::default();
        let mut report = ImportReport::default();
        if let Some(object) = value.as_object() {
            for (key, raw) in object {
                // Métadonnées et autres parties d'un thème posé à plat : pas des couleurs.
                if matches!(key.as_str(), "format" | "version" | "name" | "images" | "glass_blur") {
                    continue;
                }
                match (ThemeColor::from_key(key), raw.as_str().and_then(normalize_hex)) {
                    (Some(color), Some(hex)) => {
                        theme.colors.insert(color.key().to_string(), hex);
                        report.applied += 1;
                    }
                    _ => report.ignored.push(key.clone()),
                }
            }
        }
        report.missing = ThemeColor::ALL.len() - report.applied;
        (theme, report)
    }
}

/// Thème proposé tout fait dans les réglages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemePreset {
    Default,
    Light,
    Forest,
    Dusk,
}

impl ThemePreset {
    pub const ALL: [ThemePreset; 4] = [ThemePreset::Default, ThemePreset::Light, ThemePreset::Forest, ThemePreset::Dusk];

    /// Clé stable enregistrée dans `users.json` pour le thème choisi : ne
    /// pas la renommer (le compte serait remis sur le thème par défaut).
    pub fn key(self) -> &'static str {
        match self {
            ThemePreset::Default => "default",
            ThemePreset::Light => "light",
            ThemePreset::Forest => "forest",
            ThemePreset::Dusk => "dusk",
        }
    }

    pub fn from_key(key: &str) -> Option<ThemePreset> {
        ThemePreset::ALL.into_iter().find(|preset| preset.key() == key)
    }

    pub fn theme(self) -> Theme {
        use ThemeColor::*;
        match self {
            ThemePreset::Default => Theme::default(),
            ThemePreset::Light => Theme::from_pairs(&[
                (MenuBackground, "#f1f3f8"),
                (MenuHighlight, "#d9deec"),
                (MenuText, "#1e1e2f"),
                (Primary, "#3b5bdb"),
                (Accent, "#1c7ed6"),
                (Success, "#2f9e44"),
                (Danger, "#e03131"),
                (PageBackground, "#ffffff"),
                (Surface, "#ffffff"),
                (Text, "#222222"),
            ]),
            ThemePreset::Forest => Theme::from_pairs(&[
                (MenuBackground, "#1f3a2e"),
                (MenuHighlight, "#335e4a"),
                (MenuText, "#f1f7f3"),
                (Primary, "#2f6b4f"),
                (Accent, "#c08a2e"),
                (Success, "#2e8b57"),
                (Danger, "#b23a3a"),
                (PageBackground, "#f6f8f4"),
                (Surface, "#ffffff"),
                (Text, "#1f2a24"),
            ]),
            ThemePreset::Dusk => Theme::from_pairs(&[
                (MenuBackground, "#3b1f3f"),
                (MenuHighlight, "#5e3463"),
                (MenuText, "#fbeff8"),
                (Primary, "#7b3f82"),
                (Accent, "#e07a5f"),
                (Success, "#3a9d6e"),
                (Danger, "#c0392b"),
                (PageBackground, "#fbf7f9"),
                (Surface, "#ffffff"),
                (Text, "#2a1f2b"),
            ]),
        }
    }

    /// Thème prédéfini qui donne exactement ces couleurs, s'il y en a un.
    pub fn matching(theme: &Theme) -> Option<ThemePreset> {
        ThemePreset::ALL.into_iter().find(|preset| preset.theme().same_colors(theme))
    }
}

/// Thème créé ou importé par l'utilisateur, rangé dans son compte.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SavedTheme {
    pub id: usize,
    pub name: String,
    #[serde(alias = "colors")]
    pub theme: Theme,
}

/// Thème choisi par un compte : une référence, pas une copie des couleurs.
/// Si la référence ne mène plus à rien (prédéfini retiré de l'application,
/// thème perso absent du fichier), le compte est remis sur le thème par
/// défaut avec un message (voir `UserStore::reset_missing_theme`).
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeChoice {
    /// Clé d'un `ThemePreset` (gardée telle quelle même si inconnue, pour
    /// pouvoir détecter qu'elle n'existe plus).
    Preset(String),
    /// Id d'un `SavedTheme` du compte.
    Custom(usize),
}

impl Default for ThemeChoice {
    fn default() -> Self {
        ThemeChoice::Preset(ThemePreset::Default.key().to_string())
    }
}

/// Lecture tolérante : une valeur illisible devient un prédéfini inconnu,
/// donc « thème introuvable » au lieu d'une erreur de chargement.
impl<'de> Deserialize<'de> for ThemeChoice {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        let choice = if let Some(key) = value.get("preset").and_then(Value::as_str) {
            ThemeChoice::Preset(key.to_string())
        } else if let Some(id) = value.get("custom").and_then(Value::as_u64) {
            ThemeChoice::Custom(id as usize)
        } else {
            ThemeChoice::Preset(String::new())
        };
        Ok(choice)
    }
}

impl ThemeChoice {
    pub fn preset(preset: ThemePreset) -> ThemeChoice {
        ThemeChoice::Preset(preset.key().to_string())
    }

    /// Couleurs du thème choisi, ou `None` s'il n'existe plus.
    pub fn resolve(&self, saved: &[SavedTheme]) -> Option<Theme> {
        match self {
            ThemeChoice::Preset(key) => ThemePreset::from_key(key).map(ThemePreset::theme),
            ThemeChoice::Custom(id) => saved.iter().find(|t| t.id == *id).map(|t| t.theme.clone()),
        }
    }
}

/// Lecture tolérante de la liste des thèmes perso : une entrée abîmée est
/// ignorée au lieu d'empêcher le chargement de tous les comptes.
pub fn deserialize_saved_themes<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<SavedTheme>, D::Error> {
    let value = Value::deserialize(deserializer)?;
    let entries = value.as_array().cloned().unwrap_or_default();
    Ok(entries.into_iter().filter_map(|entry| serde_json::from_value(entry).ok()).collect())
}

/// Partie du nom de fichier à l'export, tirée du nom du thème :
/// « Forêt d'été » → `foret-d-ete`.
pub fn file_slug(name: &str) -> String {
    let mut slug = String::new();
    for c in name.chars() {
        let c = match c {
            'à' | 'â' | 'ä' | 'á' | 'À' | 'Â' | 'Ä' | 'Á' => 'a',
            'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => 'e',
            'î' | 'ï' | 'í' | 'Î' | 'Ï' | 'Í' => 'i',
            'ô' | 'ö' | 'ó' | 'Ô' | 'Ö' | 'Ó' => 'o',
            'ù' | 'û' | 'ü' | 'ú' | 'Ù' | 'Û' | 'Ü' | 'Ú' => 'u',
            'ç' | 'Ç' => 'c',
            c => c,
        };
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug = slug.trim_end_matches('-').to_string();
    if slug.is_empty() { "theme".to_string() } else { slug }
}

/// `#rrggbb` (ou `#rrggbbaa` avec transparence) en minuscules si `value`
/// est une couleur valide ; accepte aussi les formes courtes `#rgb` /
/// `#rgba` et les majuscules. Une transparence nulle (`ff`) est retirée.
pub fn normalize_hex(value: &str) -> Option<String> {
    let value = value.trim();
    let digits = value.strip_prefix('#')?;
    if !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let digits = match digits.len() {
        6 | 8 => digits.to_ascii_lowercase(),
        3 | 4 => digits.chars().flat_map(|c| [c, c]).collect::<String>().to_ascii_lowercase(),
        _ => return None,
    };
    let digits = digits.strip_suffix("ff").filter(|d| d.len() == 6).unwrap_or(&digits);
    Some(format!("#{digits}"))
}

/// Sépare une couleur normalisée en `#rrggbb` et opacité (0 à 100 %).
pub fn split_alpha(hex: &str) -> (String, u8) {
    match hex.get(7..9).and_then(|a| u8::from_str_radix(a, 16).ok()) {
        Some(alpha) => (hex[..7].to_string(), ((alpha as u32 * 100 + 127) / 255) as u8),
        None => (hex.get(..7).unwrap_or(hex).to_string(), 100),
    }
}

/// Recompose `#rrggbb` + opacité (0 à 100 %) en couleur normalisée.
pub fn with_alpha(rgb: &str, opacity: u8) -> String {
    let alpha = (opacity.min(100) as u32 * 255 + 50) / 100;
    if alpha >= 255 { rgb.to_string() } else { format!("{rgb}{alpha:02x}") }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_keeps_valid_colors_and_ignores_the_rest() {
        let text = r##"{ "name": "x", "colors": {
            "menu_background": "#ABC", "primary": "pas une couleur",
            "ancienne_couleur": "#123456", "accent": 42 } }"##;
        let ImportedTheme { name, theme, report, .. } = Theme::import_json(text).unwrap();
        assert_eq!(name.as_deref(), Some("x"));
        assert_eq!(theme.get(ThemeColor::MenuBackground), "#aabbcc");
        assert_eq!(theme.get(ThemeColor::Primary), ThemeColor::Primary.default_value());
        assert_eq!(report.applied, 1);
        assert_eq!(report.ignored.len(), 3);
        assert_eq!(report.missing, ThemeColor::ALL.len() - 1);
    }

    #[test]
    fn import_accepts_flat_object_and_rejects_garbage() {
        assert!(Theme::import_json(r##"{ "text": "#000000" }"##).is_ok());
        assert!(Theme::import_json("pas du json").is_err());
        assert!(Theme::import_json("[1, 2]").is_err());
        assert!(Theme::import_json(r##"{ "inconnu": "#000000" }"##).is_err());
    }

    #[test]
    fn export_then_import_round_trips() {
        let theme = ThemePreset::Forest.theme();
        let imported = Theme::import_json(&theme.export_json("Forêt", &BTreeMap::new())).unwrap();
        assert!(imported.theme.same_colors(&theme));
        assert!(imported.report.ignored.is_empty());
        assert_eq!(imported.name.as_deref(), Some("Forêt"));
    }

    #[test]
    fn stored_theme_with_bad_values_still_loads() {
        let theme: Theme = serde_json::from_str(r##"{ "menu_text": 3, "text": "#000000" }"##).unwrap();
        assert_eq!(theme.get(ThemeColor::Text), "#000000");
        assert_eq!(theme.get(ThemeColor::MenuText), ThemeColor::MenuText.default_value());
        let theme: Theme = serde_json::from_str("null").unwrap();
        assert!(theme.same_colors(&Theme::default()));
    }

    #[test]
    fn missing_theme_choice_is_detected() {
        let saved = vec![SavedTheme { id: 4, name: "A".into(), theme: Theme::default() }];
        assert!(ThemeChoice::Custom(4).resolve(&saved).is_some());
        assert!(ThemeChoice::Custom(5).resolve(&saved).is_none());
        assert!(ThemeChoice::Preset("forest".into()).resolve(&saved).is_some());
        assert!(ThemeChoice::Preset("supprime".into()).resolve(&saved).is_none());
        // Valeur illisible dans le fichier : lue comme « introuvable ».
        let choice: ThemeChoice = serde_json::from_str("42").unwrap();
        assert!(choice.resolve(&saved).is_none());
    }

    #[test]
    fn category_texts_follow_menu_text_until_chosen() {
        let mut theme = Theme::default();
        theme.set(ThemeColor::MenuText, "#102030");
        assert_eq!(theme.get(ThemeColor::CategoryText), "#102030");
        assert_eq!(theme.get(ThemeColor::SubcategoryText), "#102030cc");
        theme.set(ThemeColor::CategoryText, "#ff0000");
        assert_eq!(theme.get(ThemeColor::CategoryText), "#ff0000");
        assert_eq!(theme.get(ThemeColor::CategoryBackground), "#00000000");
    }

    #[test]
    fn transparency_round_trips() {
        assert_eq!(normalize_hex("#11223380").as_deref(), Some("#11223380"));
        assert_eq!(normalize_hex("#112233FF").as_deref(), Some("#112233"));
        assert_eq!(normalize_hex("#1238").as_deref(), Some("#11223388"));
        assert_eq!(split_alpha("#11223380"), ("#112233".to_string(), 50));
        assert_eq!(split_alpha("#112233"), ("#112233".to_string(), 100));
        assert_eq!(with_alpha("#112233", 50), "#11223380");
        assert_eq!(with_alpha("#112233", 100), "#112233");
    }

    #[test]
    fn images_survive_export_and_import() {
        let gif = b"GIF89a-contenu".to_vec();
        let mut theme = Theme::default();
        theme.set_image(ThemeImageSlot::Category, Some(ThemeImage::new("00ff.gif".into())));
        theme.set_glass_blur(8);
        let mut bytes = BTreeMap::new();
        bytes.insert("category".to_string(), gif.clone());

        let with = Theme::import_json(&theme.export_json("x", &bytes)).unwrap();
        assert_eq!(with.images.len(), 1);
        assert_eq!(with.images[0].bytes, gif);
        assert_eq!(with.theme.glass_blur(), 8);

        let without = Theme::import_json(&theme.export_json("x", &BTreeMap::new())).unwrap();
        assert!(without.images.is_empty());
    }

    #[test]
    fn broken_images_are_ignored_on_import() {
        let text = r##"{ "colors": { "text": "#000000" }, "images": {
            "category": { "data": "pas du base64 !" },
            "zone_supprimee": { "data": "R0lGODlh" },
            "page_background": { "data": "aGVsbG8=" } } }"##;
        let imported = Theme::import_json(text).unwrap();
        assert!(imported.images.is_empty());
        assert_eq!(imported.report.images_ignored.len(), 3);
    }

    #[test]
    fn old_flat_theme_still_loads() {
        let theme: Theme = serde_json::from_str(r##"{ "text": "#000000" }"##).unwrap();
        assert_eq!(theme.get(ThemeColor::Text), "#000000");
        let saved: SavedTheme = serde_json::from_str(r##"{ "id": 1, "name": "a", "colors": { "text": "#000000" } }"##).unwrap();
        assert_eq!(saved.theme.get(ThemeColor::Text), "#000000");
    }

    #[test]
    fn slug_is_file_safe() {
        assert_eq!(file_slug("Forêt d'été !"), "foret-d-ete");
        assert_eq!(file_slug("***"), "theme");
    }
}
