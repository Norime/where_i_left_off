use serde::{Deserialize, Serialize};

/// Un compte utilisateur local. `password_hash` vaut `None` si le compte
/// n'est pas protégé par mot de passe. `avatar_url` est réservé pour une
/// future gestion de photo de profil (toujours `None` pour l'instant,
/// aucun formulaire ne le renseigne encore).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub id: usize,
    pub username: String,
    pub password_hash: Option<String>,
    pub avatar_url: Option<String>,
}