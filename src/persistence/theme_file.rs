use std::fs;
use std::io;
use std::path::PathBuf;

/// Écrit un thème exporté dans le dossier Téléchargements (ou le dossier
/// personnel s'il n'existe pas), sans écraser un fichier existant :
/// `theme-foret.json`, puis `theme-foret-2.json`, etc. Retourne le chemin.
pub fn export_theme(slug: &str, contents: &str) -> io::Result<PathBuf> {
    let dir = dirs::download_dir()
        .or_else(dirs::home_dir)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no download directory"))?;
    fs::create_dir_all(&dir)?;

    let mut path = dir.join(format!("theme-{slug}.json"));
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("theme-{slug}-{n}.json"));
        n += 1;
    }
    fs::write(&path, contents)?;
    Ok(path)
}
