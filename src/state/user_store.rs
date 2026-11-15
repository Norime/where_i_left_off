use serde::{Deserialize, Serialize};
use crate::auth::hash_password;
use crate::models::{AvatarCrop, SavedTheme, Theme, ThemeChoice, ThemeColor, ThemePreset, User};
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
        self.users.push(User {
            id,
            username,
            password_hash,
            avatar_url: None,
            avatar_file: None,
            avatar_crop: Default::default(),
            login_background: None,
            language,
            theme_choice: ThemeChoice::default(),
            saved_themes: Vec::new(),
            legacy_theme: None,
        });
        Ok(id)
    }

    /// Supprime un compte de la liste. Ne touche pas aux fichiers déjà
    /// écrits sur le disque, voir `persistence::delete_user_data`.
    pub fn remove_user(&mut self, user_id: usize) {
        self.users.retain(|u| u.id != user_id);
    }

    /// Renomme un compte. Refuse si le nouveau nom est déjà pris par un
    /// autre compte.
    pub fn update_username(&mut self, user_id: usize, username: String) -> Result<(), UsernameTaken> {
        let username = username.trim().to_string();

        if self.users.iter().any(|u| u.id != user_id && u.username.eq_ignore_ascii_case(&username)) {
            return Err(UsernameTaken);
        }

        if let Some(user) = self.users.iter_mut().find(|u| u.id == user_id) {
            user.username = username;
        }

        Ok(())
    }

    /// Change (ou retire, avec `None`) l'icône d'un compte : nom du fichier
    /// copié par `persistence::save_profile_image`. Remplace l'ancienne icône par
    /// URL ; le cadrage repart du centre.
    pub fn set_avatar_file(&mut self, user_id: usize, file: Option<String>) {
        if let Some(user) = self.users.iter_mut().find(|u| u.id == user_id) {
            user.avatar_file = file;
            user.avatar_url = None;
            user.avatar_crop = AvatarCrop::default();
        }
    }

    /// Change (ou retire, avec `None`) le fond de l'écran de connexion du
    /// profil : nom du fichier copié par `persistence::save_profile_image`.
    pub fn set_login_background(&mut self, user_id: usize, file: Option<String>) {
        if let Some(user) = self.users.iter_mut().find(|u| u.id == user_id) {
            user.login_background = file;
        }
    }

    /// Change la partie de l'icône affichée dans le cercle.
    pub fn set_avatar_crop(&mut self, user_id: usize, crop: AvatarCrop) {
        if let Some(user) = self.users.iter_mut().find(|u| u.id == user_id) {
            user.avatar_crop = crop.clamped();
        }
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

    /// Convertit les données des anciennes versions, à appeler juste après
    /// le chargement : les couleurs enregistrées directement dans le compte
    /// deviennent le prédéfini correspondant, ou un thème perso « Mon thème ».
    pub fn migrate(&mut self) {
        for user in &mut self.users {
            let Some(colors) = user.legacy_theme.take() else { continue };
            user.theme_choice = match ThemePreset::matching(&colors) {
                Some(preset) => ThemeChoice::preset(preset),
                None => {
                    let id = next_theme_id(&user.saved_themes);
                    user.saved_themes.push(SavedTheme { id, name: "Mon thème".to_string(), theme: colors });
                    ThemeChoice::Custom(id)
                }
            };
        }
    }

    fn user_mut(&mut self, user_id: usize) -> Option<&mut User> {
        self.users.iter_mut().find(|u| u.id == user_id)
    }

    /// Choisit le thème de l'interface (appliqué immédiatement).
    pub fn select_theme(&mut self, user_id: usize, choice: ThemeChoice) {
        if let Some(user) = self.user_mut(user_id) {
            user.theme_choice = choice;
        }
    }

    /// Ajoute un thème perso (créé ou importé), le choisit et retourne son id.
    pub fn add_saved_theme(&mut self, user_id: usize, name: String, theme: Theme) -> Option<usize> {
        let user = self.user_mut(user_id)?;
        let id = next_theme_id(&user.saved_themes);
        let name = name.trim().to_string();
        user.saved_themes.push(SavedTheme { id, name, theme });
        user.theme_choice = ThemeChoice::Custom(id);
        Some(id)
    }

    /// Renomme un thème perso (un nom vide est ignoré).
    pub fn rename_saved_theme(&mut self, user_id: usize, theme_id: usize, name: String) {
        let name = name.trim().to_string();
        if name.is_empty() {
            return;
        }
        if let Some(theme) = self.user_mut(user_id).and_then(|u| u.saved_themes.iter_mut().find(|t| t.id == theme_id)) {
            theme.name = name;
        }
    }

    /// Modifie un thème perso (les prédéfinis ne sont pas modifiables).
    pub fn edit_saved_theme(&mut self, user_id: usize, theme_id: usize, edit: impl FnOnce(&mut Theme)) {
        if let Some(saved) = self.user_mut(user_id).and_then(|u| u.saved_themes.iter_mut().find(|t| t.id == theme_id)) {
            edit(&mut saved.theme);
        }
    }

    /// Change une couleur d'un thème perso.
    pub fn set_saved_theme_color(&mut self, user_id: usize, theme_id: usize, color: ThemeColor, value: &str) {
        self.edit_saved_theme(user_id, theme_id, |theme| theme.set(color, value));
    }

    /// Supprime un thème perso ; s'il était utilisé, le compte repasse sur
    /// le thème par défaut.
    pub fn delete_saved_theme(&mut self, user_id: usize, theme_id: usize) {
        if let Some(user) = self.user_mut(user_id) {
            user.saved_themes.retain(|t| t.id != theme_id);
            if user.theme_choice == ThemeChoice::Custom(theme_id) {
                user.theme_choice = ThemeChoice::default();
            }
        }
    }

    /// Fichiers d'images utilisés par les thèmes perso d'un compte.
    pub fn used_media(&self, user_id: usize) -> std::collections::HashSet<String> {
        self.users
            .iter()
            .filter(|u| u.id == user_id)
            .flat_map(|u| u.saved_themes.iter())
            .flat_map(|saved| saved.theme.images().into_iter().map(|(_, image)| image.file.clone()))
            .collect()
    }

    /// Si le thème choisi n'existe plus, remet le thème par défaut et
    /// retourne `true` (pour prévenir l'utilisateur).
    pub fn reset_missing_theme(&mut self, user_id: usize) -> bool {
        match self.user_mut(user_id) {
            Some(user) if user.theme_missing() => {
                user.theme_choice = ThemeChoice::default();
                true
            }
            _ => false,
        }
    }
}

fn next_theme_id(themes: &[SavedTheme]) -> usize {
    themes.iter().map(|t| t.id + 1).max().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store_from(json: &str) -> UserStore {
        let mut store: UserStore = serde_json::from_str(json).unwrap();
        store.migrate();
        store
    }

    #[test]
    fn old_colors_become_a_custom_theme() {
        let store = store_from(r##"{ "users": [{ "id": 0, "username": "a", "password_hash": null,
            "avatar_url": null, "theme": { "text": "#000000" } }], "next_user_id": 1 }"##);
        let user = &store.users[0];
        assert_eq!(user.saved_themes.len(), 1);
        assert_eq!(user.theme_choice, ThemeChoice::Custom(user.saved_themes[0].id));
        assert_eq!(user.effective_theme().get(ThemeColor::Text), "#000000");
    }

    #[test]
    fn missing_theme_is_reset_to_default() {
        let mut store = store_from(r##"{ "users": [{ "id": 0, "username": "a", "password_hash": null,
            "avatar_url": null, "theme_choice": { "custom": 7 },
            "saved_themes": [{ "id": 1, "name": "ok", "colors": {} }, "abîmé"] }], "next_user_id": 1 }"##);
        assert_eq!(store.users[0].saved_themes.len(), 1);
        assert!(store.reset_missing_theme(0));
        assert_eq!(store.users[0].theme_choice, ThemeChoice::default());
        assert!(!store.reset_missing_theme(0));
    }

    #[test]
    fn deleting_the_active_theme_falls_back_to_default() {
        let mut store = UserStore::default();
        let user_id = store.add_user("a".into(), None, Language::Fr).unwrap();
        let theme_id = store.add_saved_theme(user_id, "Mien".into(), Theme::default()).unwrap();
        store.delete_saved_theme(user_id, theme_id);
        assert_eq!(store.users[0].theme_choice, ThemeChoice::default());
    }
}
