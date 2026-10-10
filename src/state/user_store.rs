use serde::{Deserialize, Serialize};
use crate::auth::hash_password;
use crate::models::User;
use crate::i18n::Language;

/// Le nom d'utilisateur est déjà pris par un autre compte.
#[derive(Debug)]
pub struct UsernameTaken;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct UserStore {
    pub users: Vec<User>,
    next_user_id: usize,
    /// Langue de l'écran de connexion : celle du dernier compte connecté,
    /// ou celle choisie avec le sélecteur de cet écran.
    #[serde(default)]
    pub last_language: Language,
}

impl UserStore {
    /// Crée un nouveau compte si le nom d'utilisateur n'est pas déjà pris.
    /// Hache le mot de passe s'il y en a un. Retourne l'id créé, ou un
    /// message d'erreur si le nom existe déjà.
    pub fn add_user(&mut self, username: String, password: Option<String>, language: Language) -> Result<usize, UsernameTaken> {
        let username = username.trim().to_string();

        if self.users.iter().any(|u| u.username.eq_ignore_ascii_case(&username)) {
            return Err(UsernameTaken);
        }

        let password_hash = password.map(|p| hash_password(&p));
        let id = self.next_user_id;
        self.next_user_id += 1;
        self.users.push(User { id, username, password_hash, avatar_url: None, language });
        Ok(id)
    }

    /// Supprime un compte de la liste. Ne touche pas aux fichiers déjà
    /// écrits sur le disque, voir `persistence::delete_user_data`.
    pub fn remove_user(&mut self, user_id: usize) {
        self.users.retain(|u| u.id != user_id);
    }

    /// Met à jour le nom d'utilisateur et l'image de profil d'un compte.
    /// Refuse si le nouveau nom est déjà pris par un autre compte.
    pub fn update_user(&mut self, user_id: usize, username: String, avatar_url: Option<String>) -> Result<(), UsernameTaken> {
        let username = username.trim().to_string();

        if self.users.iter().any(|u| u.id != user_id && u.username.eq_ignore_ascii_case(&username)) {
            return Err(UsernameTaken);
        }

        if let Some(user) = self.users.iter_mut().find(|u| u.id == user_id) {
            user.username = username;
            user.avatar_url = avatar_url;
        }

        Ok(())
    }

    /// Change la langue de l'interface d'un compte (appliquée immédiatement).
    pub fn set_language(&mut self, user_id: usize, language: Language) {
        if let Some(user) = self.users.iter_mut().find(|u| u.id == user_id) {
            user.language = language;
        }
    }

    /// Définit ou remplace le mot de passe d'un compte.
    pub fn set_password(&mut self, user_id: usize, password: String) {
        if let Some(user) = self.users.iter_mut().find(|u| u.id == user_id) {
            user.password_hash = Some(hash_password(&password));
        }
    }

    /// Retire le mot de passe d'un compte (accessible sans mot de passe ensuite).
    pub fn remove_password(&mut self, user_id: usize) {
        if let Some(user) = self.users.iter_mut().find(|u| u.id == user_id) {
            user.password_hash = None;
        }
    }
}