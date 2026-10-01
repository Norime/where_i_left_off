use serde::{Deserialize, Serialize};
use crate::auth::hash_password;
use crate::models::User;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct UserStore {
    pub users: Vec<User>,
    next_user_id: usize,
}

impl UserStore {
    /// Crée un nouveau compte si le nom d'utilisateur n'est pas déjà pris.
    /// Hache le mot de passe s'il y en a un. Retourne l'id créé, ou un
    /// message d'erreur si le nom existe déjà.
    pub fn add_user(&mut self, username: String, password: Option<String>) -> Result<usize, String> {
        let username = username.trim().to_string();

        if self.users.iter().any(|u| u.username.eq_ignore_ascii_case(&username)) {
            return Err("Ce nom d'utilisateur existe déjà.".to_string());
        }

        let password_hash = password.map(|p| hash_password(&p));
        let id = self.next_user_id;
        self.next_user_id += 1;
        self.users.push(User { id, username, password_hash, avatar_url: None });
        Ok(id)
    }

    /// Supprime un compte de la liste. Ne touche pas aux fichiers déjà
    /// écrits sur le disque, voir `persistence::delete_user_data`.
    pub fn remove_user(&mut self, user_id: usize) {
        self.users.retain(|u| u.id != user_id);
    }

    /// Met à jour le nom d'utilisateur et l'image de profil d'un compte.
    /// Refuse si le nouveau nom est déjà pris par un autre compte.
    pub fn update_user(&mut self, user_id: usize, username: String, avatar_url: Option<String>) -> Result<(), String> {
        let username = username.trim().to_string();

        if self.users.iter().any(|u| u.id != user_id && u.username.eq_ignore_ascii_case(&username)) {
            return Err("Ce nom d'utilisateur existe déjà.".to_string());
        }

        if let Some(user) = self.users.iter_mut().find(|u| u.id == user_id) {
            user.username = username;
            user.avatar_url = avatar_url;
        }

        Ok(())
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