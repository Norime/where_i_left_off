use std::fs;
use std::path::PathBuf;

use crate::state::SourceStore;

/// ⚠️ Remplace "dioxus_library_app" par le même nom de dossier que celui
/// utilisé dans tes autres fichiers de persistance (categories.json,
/// items.json...) pour que sources.json atterrisse au même endroit.
fn sources_file_path(user_id: usize) -> Option<PathBuf> {
    let mut path = dirs::data_dir()?;
    path.push("dioxus_library_app");
    path.push("users");
    path.push(user_id.to_string());
    fs::create_dir_all(&path).ok()?;
    path.push("sources.json");
    Some(path)
}

pub fn load_sources(user_id: usize) -> SourceStore {
    sources_file_path(user_id)
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

pub fn save_sources(user_id: usize, store: &SourceStore) {
    if let Some(path) = sources_file_path(user_id) {
        if let Ok(json) = serde_json::to_string_pretty(store) {
            let _ = fs::write(path, json);
        }
    }
}