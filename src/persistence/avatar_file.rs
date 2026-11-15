use std::fs;
use std::path::PathBuf;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;

use crate::models::{detect_image_type, is_valid_media_name, media_file_name, User, MAX_IMAGE_BYTES};
use super::{MediaError, APP_DIR_NAME};

/// Dossier de l'icône d'un compte. L'image choisie y est copiée : elle reste
/// donc affichée même si le fichier d'origine est déplacé ou supprimé. Le
/// dossier part avec le compte (`delete_user_data`).
fn avatar_dir(user_id: usize) -> PathBuf {
    let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push(APP_DIR_NAME);
    path.push("users");
    path.push(user_id.to_string());
    path.push("avatar");
    path
}

/// Copie l'image choisie comme icône du compte et retourne son nom de
/// fichier. L'ancienne icône est supprimée : il n'y en a qu'une par compte.
pub fn save_avatar(user_id: usize, bytes: &[u8]) -> Result<String, MediaError> {
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(MediaError::TooLarge);
    }
    let name = media_file_name(bytes).ok_or(MediaError::Unsupported)?;
    let dir = avatar_dir(user_id);
    fs::create_dir_all(&dir).map_err(|e| MediaError::Io(e.to_string()))?;
    fs::write(dir.join(&name), bytes).map_err(|e| MediaError::Io(e.to_string()))?;
    remove_other_avatars(user_id, Some(&name));
    Ok(name)
}

/// Supprime les fichiers d'icône du compte, sauf `keep`.
pub fn remove_other_avatars(user_id: usize, keep: Option<&str>) {
    let Ok(entries) = fs::read_dir(avatar_dir(user_id)) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if is_valid_media_name(&name) && Some(name.as_str()) != keep {
            let _ = fs::remove_file(entry.path());
        }
    }
}

/// Image à afficher pour un compte (valeur de `background-image`), ou
/// `None` s'il n'en a pas. L'icône copiée est envoyée en data URI ; une
/// ancienne icône par URL (avant les fichiers locaux) reste affichée tant
/// qu'elle n'est pas remplacée. Une icône introuvable (fichier supprimé du
/// dossier) donne simplement le cercle gris par défaut.
pub fn avatar_css_url(user: &User) -> Option<String> {
    if let Some(name) = user.avatar_file.as_deref().filter(|name| is_valid_media_name(name)) {
        let bytes = fs::read(avatar_dir(user.id).join(name)).ok()?;
        let (mime, _) = detect_image_type(&bytes)?;
        return Some(format!("url(\"data:{mime};base64,{}\")", BASE64.encode(&bytes)));
    }
    user.avatar_url
        .as_deref()
        .filter(|url| !url.contains(['\'', '"', '(', ')']))
        .map(|url| format!("url('{url}')"))
}
