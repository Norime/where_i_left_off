use std::fs;
use std::path::PathBuf;
use crate::state::UniverseStore;
use super::APP_DIR_NAME;

/// Construit le chemin du fichier d'univers propre à un utilisateur.
fn data_file_path(user_id: usize) -> PathBuf {
    let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push(APP_DIR_NAME);
    path.push("users");
    path.push(user_id.to_string());
    path.push("universes.json");
    path
}

/// Charge les univers de l'utilisateur, ou aucun univers si
/// le fichier n'existe pas encore.
pub fn load_universes(user_id: usize) -> UniverseStore {
    let path = data_file_path(user_id);

    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => UniverseStore::default(),
    }
}

/// Sauvegarde les univers de l'utilisateur sur le disque.
pub fn save_universes(user_id: usize, store: &UniverseStore) {
    let path = data_file_path(user_id);

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    if let Ok(json) = serde_json::to_string_pretty(store) {
        let _ = fs::write(&path, json);
    }
}