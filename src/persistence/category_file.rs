use std::fs;
use std::path::PathBuf;
use crate::state::CategoryStore;
use super::APP_DIR_NAME;

/// Construit le chemin du fichier de catégories propre à un utilisateur.
fn data_file_path(user_id: usize) -> PathBuf {
    let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push(APP_DIR_NAME);
    path.push("users");
    path.push(user_id.to_string());
    path.push("categories.json");
    path
}

/// Charge les catégories de l'utilisateur, ou un store par défaut si
/// le fichier n'existe pas encore.
pub fn load_categories(user_id: usize) -> CategoryStore {
    let path = data_file_path(user_id);

    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| CategoryStore::new()),
        Err(_) => CategoryStore::new(),
    }
}

/// Sauvegarde les catégories de l'utilisateur sur le disque.
pub fn save_categories(user_id: usize, store: &CategoryStore) {
    let path = data_file_path(user_id);

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    if let Ok(json) = serde_json::to_string_pretty(store) {
        let _ = fs::write(&path, json);
    }
}