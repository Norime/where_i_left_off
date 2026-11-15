use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::PathBuf;

use crate::models::{is_valid_media_name, media_file_name, EmbeddedImage, Theme, ThemeImage, MAX_IMAGE_BYTES};
use super::APP_DIR_NAME;

/// Dossier des images de thème d'un utilisateur : les fichiers choisis sont
/// copiés ici, le thème reste donc intact si l'original est déplacé.
fn media_dir(user_id: usize) -> PathBuf {
    let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push(APP_DIR_NAME);
    path.push("users");
    path.push(user_id.to_string());
    path.push("theme-media");
    path
}

/// Pourquoi une image n'a pas pu être ajoutée.
#[derive(Debug, PartialEq)]
pub enum MediaError {
    /// Pas une image, ou un format non pris en charge.
    Unsupported,
    TooLarge,
    Io(String),
}

/// Copie une image dans le dossier du compte et retourne son nom de fichier.
pub fn save_media(user_id: usize, bytes: &[u8]) -> Result<String, MediaError> {
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(MediaError::TooLarge);
    }
    let name = media_file_name(bytes).ok_or(MediaError::Unsupported)?;
    let dir = media_dir(user_id);
    fs::create_dir_all(&dir).map_err(|e| MediaError::Io(e.to_string()))?;
    let path = dir.join(&name);
    // Même contenu = même nom : déjà présente, rien à écrire.
    if !path.exists() {
        fs::write(&path, bytes).map_err(|e| MediaError::Io(e.to_string()))?;
    }
    Ok(name)
}

/// Contenu d'une image du compte, `None` si elle a disparu ou si le nom est
/// suspect (fichier modifié à la main).
pub fn load_media(user_id: usize, name: &str) -> Option<Vec<u8>> {
    if !is_valid_media_name(name) {
        return None;
    }
    fs::read(media_dir(user_id).join(name)).ok()
}

/// Taille totale des images d'un thème (affichée avant l'export).
pub fn media_size(user_id: usize, theme: &Theme) -> u64 {
    theme
        .images()
        .iter()
        .filter(|(_, image)| is_valid_media_name(&image.file))
        .filter_map(|(_, image)| fs::metadata(media_dir(user_id).join(&image.file)).ok())
        .map(|metadata| metadata.len())
        .sum()
}

/// Contenu des images d'un thème pour l'export (clé de zone → contenu).
/// Les images introuvables sont simplement absentes.
pub fn load_theme_images(user_id: usize, theme: &Theme) -> BTreeMap<String, Vec<u8>> {
    theme
        .images()
        .into_iter()
        .filter_map(|(slot, image)| load_media(user_id, &image.file).map(|bytes| (slot.key().to_string(), bytes)))
        .collect()
}

/// Enregistre les images d'un thème importé et les ajoute au thème.
/// Retourne le nombre d'images qui n'ont pas pu être enregistrées.
pub fn save_imported_images(user_id: usize, theme: &mut Theme, images: Vec<EmbeddedImage>) -> usize {
    let mut failed = 0;
    for embedded in images {
        match save_media(user_id, &embedded.bytes) {
            Ok(file) => theme.set_image(embedded.slot, Some(ThemeImage { file, fit: embedded.fit, opacity: embedded.opacity, blur: embedded.blur })),
            Err(_) => failed += 1,
        }
    }
    failed
}

/// Supprime les images qui ne sont plus utilisées par aucun thème du compte
/// (thème supprimé, image remplacée).
pub fn remove_unused_media(user_id: usize, used: &HashSet<String>) {
    let Ok(entries) = fs::read_dir(media_dir(user_id)) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if is_valid_media_name(&name) && !used.contains(&name) {
            let _ = fs::remove_file(entry.path());
        }
    }
}
