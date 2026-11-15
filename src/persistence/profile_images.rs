use std::fs;
use std::path::PathBuf;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;

use crate::models::{detect_image_type, is_valid_media_name, media_file_name, User, MAX_IMAGE_BYTES};
use super::{MediaError, APP_DIR_NAME};

/// Images propres à un profil, réglées dans Apparence › Profil.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileImage {
    /// Icône du compte.
    Avatar,
    /// Fond de l'écran de connexion quand ce profil est survolé ou choisi.
    LoginBackground,
}

impl ProfileImage {
    fn dir_name(self) -> &'static str {
        match self {
            ProfileImage::Avatar => "avatar",
            ProfileImage::LoginBackground => "login-background",
        }
    }
}

/// Dossier d'une image du profil. L'image choisie y est copiée : elle reste
/// donc affichée même si le fichier d'origine est déplacé ou supprimé. Le
/// dossier part avec le compte (`delete_user_data`).
fn image_dir(user_id: usize, kind: ProfileImage) -> PathBuf {
    let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push(APP_DIR_NAME);
    path.push("users");
    path.push(user_id.to_string());
    path.push(kind.dir_name());
    path
}

/// Copie l'image choisie et retourne son nom de fichier. L'ancienne image
/// du même type est supprimée : il n'y en a qu'une par profil.
pub fn save_profile_image(user_id: usize, kind: ProfileImage, bytes: &[u8]) -> Result<String, MediaError> {
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(MediaError::TooLarge);
    }
    let name = media_file_name(bytes).ok_or(MediaError::Unsupported)?;
    let dir = image_dir(user_id, kind);
    fs::create_dir_all(&dir).map_err(|e| MediaError::Io(e.to_string()))?;
    fs::write(dir.join(&name), bytes).map_err(|e| MediaError::Io(e.to_string()))?;
    remove_profile_images(user_id, kind, Some(&name));
    Ok(name)
}

/// Supprime les fichiers d'un type d'image du profil, sauf `keep`.
pub fn remove_profile_images(user_id: usize, kind: ProfileImage, keep: Option<&str>) {
    let Ok(entries) = fs::read_dir(image_dir(user_id, kind)) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if is_valid_media_name(&name) && Some(name.as_str()) != keep {
            let _ = fs::remove_file(entry.path());
        }
    }
}

/// Image copiée, en data URI pour `background-image`, ou `None` si elle
/// est absente ou introuvable (fichier supprimé du dossier).
fn stored_css_url(user_id: usize, kind: ProfileImage, name: Option<&str>) -> Option<String> {
    let name = name.filter(|name| is_valid_media_name(name))?;
    let bytes = fs::read(image_dir(user_id, kind).join(name)).ok()?;
    let (mime, _) = detect_image_type(&bytes)?;
    Some(format!("url(\"data:{mime};base64,{}\")", BASE64.encode(&bytes)))
}

/// Icône d'un compte (valeur de `background-image`), ou `None` s'il n'en a
/// pas. Une ancienne icône par URL (avant les fichiers locaux) reste
/// affichée tant qu'elle n'est pas remplacée. Une icône introuvable donne
/// simplement le fond gris par défaut.
pub fn avatar_css_url(user: &User) -> Option<String> {
    if user.avatar_file.is_some() {
        return stored_css_url(user.id, ProfileImage::Avatar, user.avatar_file.as_deref());
    }
    user.avatar_url
        .as_deref()
        .filter(|url| !url.contains(['\'', '"', '(', ')']))
        .map(|url| format!("url('{url}')"))
}

/// Fond de l'écran de connexion d'un profil, ou `None` s'il n'en a pas.
pub fn login_background_css_url(user: &User) -> Option<String> {
    stored_css_url(user.id, ProfileImage::LoginBackground, user.login_background.as_deref())
}
