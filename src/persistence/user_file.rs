use std::fs;
use std::path::PathBuf;
use crate::state::UserStore;
use super::APP_DIR_NAME;

/// Construit le chemin du fichier listant tous les comptes locaux.
fn users_file_path() -> PathBuf {
    let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push(APP_DIR_NAME);
    path.push("users.json");
    path
}

/// Charge la liste des comptes, ou un store vide si le fichier n'existe
/// pas encore.
pub fn load_users() -> UserStore {
    let path = users_file_path();

    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => UserStore::default(),
    }
}

/// Sauvegarde la liste des comptes sur le disque.
pub fn save_users(store: &UserStore) {
    let path = users_file_path();

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    if let Ok(json) = serde_json::to_string_pretty(store) {
        let _ = fs::write(&path, json);
    }
}

/// Supprime tout le dossier de données d'un utilisateur (catégories,
/// éléments...) après suppression de son compte.
pub fn delete_user_data(user_id: usize) {
    let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push(APP_DIR_NAME);
    path.push("users");
    path.push(user_id.to_string());
    let _ = fs::remove_dir_all(&path);
}