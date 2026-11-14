use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

/// Endroit de l'interface qui peut recevoir une image de fond (PNG, JPEG,
/// GIF animé, WebP…). Même logique que `ThemeColor` : clé stable dans les
/// fichiers, une clé inconnue est ignorée.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeImageSlot {
    PageBackground,
    MenuBackground,
    Category,
    Subcategory,
}

impl ThemeImageSlot {
    pub const ALL: [ThemeImageSlot; 4] = [
        ThemeImageSlot::PageBackground,
        ThemeImageSlot::MenuBackground,
        ThemeImageSlot::Category,
        ThemeImageSlot::Subcategory,
    ];

    /// Clé stable dans les fichiers : ne pas la renommer.
    pub fn key(self) -> &'static str {
        match self {
            ThemeImageSlot::PageBackground => "page_background",
            ThemeImageSlot::MenuBackground => "menu_background",
            ThemeImageSlot::Category => "category",
            ThemeImageSlot::Subcategory => "subcategory",
        }
    }

    pub fn from_key(key: &str) -> Option<ThemeImageSlot> {
        ThemeImageSlot::ALL.into_iter().find(|slot| slot.key() == key)
    }

    /// Variable CSS qui contient l'image (`url(data:…)`), définie à part
    /// pour ne pas renvoyer l'image à chaque changement de couleur.
    pub fn css_variable(self) -> String {
        format!("--theme-img-{}", self.key().replace('_', "-"))
    }

    /// Éléments qui reçoivent l'image.
    fn selector(self) -> &'static str {
        match self {
            // Calque fixe derrière tout le contenu plutôt que
            // `background-attachment: fixed` sur `body` : avec ce dernier,
            // WebKitGTK redessine le fond par morceaux pendant le défilement
            // et un GIF animé se retrouve coupé et désynchronisé.
            ThemeImageSlot::PageBackground => "body::before",
            ThemeImageSlot::MenuBackground => ".sidebar, .profile-menu",
            ThemeImageSlot::Category => ".sidebar .category-header",
            ThemeImageSlot::Subcategory => ".sidebar .subcategory-item",
        }
    }

    /// Couleur du voile posé sur l'image (pour garder le texte lisible).
    fn veil_color(self) -> &'static str {
        match self {
            ThemeImageSlot::PageBackground => "var(--page-bg)",
            _ => "var(--menu-bg)",
        }
    }

    /// Règles en plus quand l'image est présente (les lignes de catégories
    /// deviennent des tuiles pour que l'image se voie).
    fn extra_rules(self) -> &'static str {
        match self {
            // Propre couche de rendu (`translateZ`) : le GIF est toujours
            // dessiné en entier, le contenu défile simplement par-dessus.
            ThemeImageSlot::PageBackground => {
                "content: \"\"; position: fixed; inset: 0; z-index: -1; pointer-events: none; transform: translateZ(0); "
            }
            ThemeImageSlot::MenuBackground => "",
            // Les lignes sont déjà des tuiles (voir `sidebar.css`).
            ThemeImageSlot::Category | ThemeImageSlot::Subcategory => "",
        }
    }
}

/// Façon d'afficher l'image dans sa zone.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageFit {
    /// Remplit toute la zone (l'image peut être rognée).
    #[default]
    Cover,
    /// Image entière visible (des bandes peuvent apparaître).
    Contain,
    /// Mosaïque à la taille d'origine.
    Repeat,
}

impl ImageFit {
    pub const ALL: [ImageFit; 3] = [ImageFit::Cover, ImageFit::Contain, ImageFit::Repeat];

    pub fn key(self) -> &'static str {
        match self {
            ImageFit::Cover => "cover",
            ImageFit::Contain => "contain",
            ImageFit::Repeat => "repeat",
        }
    }

    pub fn from_key(key: &str) -> Option<ImageFit> {
        ImageFit::ALL.into_iter().find(|fit| fit.key() == key)
    }
}

pub const DEFAULT_IMAGE_OPACITY: u8 = 60;

/// Image d'un thème : le fichier (copié dans le dossier du compte, voir
/// `persistence::theme_media`), son affichage et son opacité.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ThemeImage {
    pub file: String,
    pub fit: ImageFit,
    /// 100 = image pleinement visible, 0 = image cachée par la couleur de
    /// la zone (posée par-dessus pour garder le texte lisible).
    pub opacity: u8,
}

impl ThemeImage {
    pub fn new(file: String) -> ThemeImage {
        ThemeImage { file, fit: ImageFit::default(), opacity: DEFAULT_IMAGE_OPACITY }
    }

    /// Lecture tolérante (fichier modifié à la main, ancienne version) :
    /// `None` si le nom de fichier est absent ou suspect.
    pub fn from_value(value: &Value) -> Option<ThemeImage> {
        let file = value.get("file")?.as_str()?;
        if !is_valid_media_name(file) {
            return None;
        }
        Some(ThemeImage {
            file: file.to_string(),
            fit: fit_from_value(value),
            opacity: opacity_from_value(value),
        })
    }

    /// Règle CSS de la zone : l'opacité est obtenue en posant la couleur de
    /// la zone par-dessus l'image (d'autant plus couvrante que l'opacité est
    /// basse). Si l'image n'est pas chargée, sa variable n'existe pas et le
    /// navigateur ignore simplement l'image (la couleur de fond reste).
    pub fn css_rule(&self, slot: ThemeImageSlot) -> String {
        let cover = 100 - self.opacity.min(100);
        let veil = format!("color-mix(in srgb, {} {cover}%, transparent)", slot.veil_color());
        let (size, repeat) = match self.fit {
            ImageFit::Cover => ("cover", "no-repeat"),
            ImageFit::Contain => ("contain", "no-repeat"),
            ImageFit::Repeat => ("auto", "repeat"),
        };
        format!(
            "{} {{ background-image: linear-gradient({veil}, {veil}), var({}); background-size: auto, {size}; \
             background-repeat: no-repeat, {repeat}; background-position: center, center; {}}}",
            slot.selector(),
            slot.css_variable(),
            slot.extra_rules(),
        )
    }
}

impl<'de> Deserialize<'de> for ThemeImage {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        ThemeImage::from_value(&value).ok_or_else(|| serde::de::Error::custom("invalid theme image"))
    }
}

pub fn fit_from_value(value: &Value) -> ImageFit {
    value.get("fit").and_then(Value::as_str).and_then(ImageFit::from_key).unwrap_or_default()
}

/// Opacité de l'image ; les fichiers d'avant enregistraient l'inverse sous
/// le nom `veil` (part de couleur posée sur l'image), converti ici.
pub fn opacity_from_value(value: &Value) -> u8 {
    let percent = |key: &str| value.get(key).and_then(Value::as_u64).map(|v| v.min(100) as u8);
    percent("opacity")
        .or_else(|| percent("veil").map(|veil| 100 - veil))
        .unwrap_or(DEFAULT_IMAGE_OPACITY)
}

/// Taille maximale d'une image de thème.
pub const MAX_IMAGE_BYTES: usize = 20 * 1024 * 1024;

/// Type d'une image d'après son contenu (pas son extension, qui peut
/// mentir) : `(type MIME, extension)`, ou `None` si le format n'est pas pris
/// en charge.
pub fn detect_image_type(bytes: &[u8]) -> Option<(&'static str, &'static str)> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        Some(("image/png", "png"))
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some(("image/jpeg", "jpg"))
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some(("image/gif", "gif"))
    } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some(("image/webp", "webp"))
    } else if bytes.starts_with(b"BM") {
        Some(("image/bmp", "bmp"))
    } else if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" && (&bytes[8..12] == b"avif" || &bytes[8..12] == b"avis") {
        Some(("image/avif", "avif"))
    } else {
        None
    }
}

/// Nom de fichier produit par `media_file_name` : protège contre un
/// fichier de thème qui pointerait ailleurs sur le disque (`../…`).
pub fn is_valid_media_name(name: &str) -> bool {
    let Some((stem, ext)) = name.split_once('.') else { return false };
    !stem.is_empty()
        && stem.len() <= 32
        && stem.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        && matches!(ext, "png" | "jpg" | "gif" | "webp" | "bmp" | "avif")
}

/// Nom de fichier d'une image : empreinte de son contenu (la même image
/// ajoutée deux fois n'est stockée qu'une fois) + extension. `None` si ce
/// n'est pas une image prise en charge.
pub fn media_file_name(bytes: &[u8]) -> Option<String> {
    let (_, ext) = detect_image_type(bytes)?;
    // FNV-1a 64 bits : stable d'une version de Rust à l'autre.
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    Some(format!("{hash:016x}.{ext}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_names_are_safe() {
        let png = [0x89, b'P', b'N', b'G', 1, 2, 3];
        let name = media_file_name(&png).unwrap();
        assert!(name.ends_with(".png"));
        assert!(is_valid_media_name(&name));
        assert!(!is_valid_media_name("../users.json"));
        assert!(!is_valid_media_name("abc.exe"));
        assert!(!is_valid_media_name(".png"));
        assert!(media_file_name(b"pas une image").is_none());
    }

    #[test]
    fn image_reading_is_lenient() {
        let value = serde_json::json!({ "file": "0a1b.gif", "fit": "inconnu", "veil": 900 });
        let image = ThemeImage::from_value(&value).unwrap();
        assert_eq!(image.fit, ImageFit::Cover);
        // Ancien `veil` (plafonné à 100) converti en opacité.
        assert_eq!(image.opacity, 0);
        let image = ThemeImage::from_value(&serde_json::json!({ "file": "0a1b.gif", "veil": 30 })).unwrap();
        assert_eq!(image.opacity, 70);
        // Fond des pages : calque fixe, jamais `background-attachment: fixed`.
        let rule = image.css_rule(ThemeImageSlot::PageBackground);
        assert!(rule.starts_with("body::before {") && rule.contains("position: fixed") && !rule.contains("attachment"));
        assert!(ThemeImage::from_value(&serde_json::json!({ "file": "/etc/passwd" })).is_none());
    }
}
