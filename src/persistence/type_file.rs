use std::fs;
use std::path::PathBuf;
use crate::state::TypeStore;
use super::APP_DIR_NAME;

fn data_file_path(user_id: usize) -> PathBuf {
    let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push(APP_DIR_NAME);
    path.push("users");
    path.push(user_id.to_string());
    path.push("types.json");
    path
}

/// Charge les types de l'utilisateur, ou un store vide si le fichier
/// n'existe pas encore.
pub fn load_types(user_id: usize) -> TypeStore {
    let path = data_file_path(user_id);

    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => TypeStore::default(),
    }
}

/// Sauvegarde les types de l'utilisateur sur le disque.
pub fn save_types(user_id: usize, store: &TypeStore) {
    let path = data_file_path(user_id);

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    if let Ok(json) = serde_json::to_string_pretty(store) {
        let _ = fs::write(&path, json);
    }
}