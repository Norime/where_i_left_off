use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::i18n::Language;

/// Nom traduit dans `language` s'il existe et n'est pas vide, sinon le nom
/// par défaut.
fn translated<'a>(name: &'a str, translations: &'a HashMap<Language, String>, language: Language) -> &'a str {
    translations
        .get(&language)
        .map(|t| t.as_str())
        .filter(|t| !t.trim().is_empty())
        .unwrap_or(name)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Subcategory {
    pub id: usize,
    pub name: String,
    /// Noms propres à chaque langue de l'interface (absents des anciens
    /// fichiers, d'où le `default`).
    #[serde(default)]
    pub translations: HashMap<Language, String>,
}

impl Subcategory {
    /// Nom à afficher dans la langue active.
    pub fn display_name(&self, language: Language) -> &str {
        translated(&self.name, &self.translations, language)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Category {
    pub id: usize,
    pub name: String,
    #[serde(default)]
    pub translations: HashMap<Language, String>,
    pub subcategories: Vec<Subcategory>,
    next_subcategory_id: usize,
}

impl Category {
    /// Crée une nouvelle catégorie avec l'id et le nom donnés, en lui ajoutant
    /// automatiquement une sous-catégorie "All" (toujours créée en premier,
    /// donc toujours à l'id 0).
    pub fn new(id: usize, name: String) -> Self {
        let mut category = Category {
            id,
            name,
            translations: HashMap::new(),
            subcategories: Vec::new(),
            next_subcategory_id: 0,
        };
        category.add_subcategory("All".to_string());
        category
    }

    /// Nom à afficher dans la langue active.
    pub fn display_name(&self, language: Language) -> &str {
        translated(&self.name, &self.translations, language)
    }

    /// Ajoute une nouvelle sous-catégorie, avec un id unique généré
    /// via le compteur interne `next_subcategory_id`.
    pub fn add_subcategory(&mut self, name: String) {
        let id = self.next_subcategory_id;
        self.next_subcategory_id += 1;
        self.subcategories.push(Subcategory { id, name, translations: HashMap::new() });
    }
}
