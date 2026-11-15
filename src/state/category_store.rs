use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::i18n::Language;
use crate::models::{Category, SubcategoryTemplate};

/// Applique un nouveau nom et de nouvelles traductions (texte nettoyé).
fn apply_names(
    name: &mut String,
    translations: &mut HashMap<Language, String>,
    new_name: String,
    new_translations: HashMap<Language, String>,
) {
    let new_name = new_name.trim();
    if !new_name.is_empty() {
        *name = new_name.to_string();
    }
    *translations = new_translations
        .into_iter()
        .map(|(lang, t)| (lang, t.trim().to_string()))
        .filter(|(_, t)| !t.is_empty())
        .collect();
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CategoryStore {
    pub categories: Vec<Category>,
    next_category_id: usize,
}

impl CategoryStore {
    /// Crée un store initial avec une catégorie "Général" par défaut.
    pub fn new() -> Self {
        let mut store = CategoryStore {
            categories: Vec::new(),
            next_category_id: 0,
        };
        store.add_category("Général".to_string());
        store
    }

    /// Ajoute une nouvelle catégorie (avec sa sous-catégorie "All" automatique)
    /// et retourne l'id qui lui a été attribué.
    pub fn add_category(&mut self, name: String) -> usize {
        let id = self.next_category_id;
        self.next_category_id += 1;
        self.categories.push(Category::new(id, name));
        id
    }

    /// Ajoute une sous-catégorie à la catégorie correspondant à `category_id`.
    pub fn add_subcategory(&mut self, category_id: usize, name: String, template: Option<SubcategoryTemplate>) {
        if let Some(category) = self.categories.iter_mut().find(|c| c.id == category_id) {
            category.add_subcategory(name, template);
        }
    }

    /// Modèle d'une sous-catégorie (« All », id 0, n'en a jamais).
    pub fn subcategory_template(&self, category_id: usize, subcategory_id: usize) -> Option<SubcategoryTemplate> {
        self.categories
            .iter()
            .find(|c| c.id == category_id)?
            .subcategories
            .iter()
            .find(|s| s.id == subcategory_id)?
            .template()
    }

    /// Change le modèle d'une sous-catégorie : seules les fiches créées
    /// ensuite sont concernées.
    pub fn set_subcategory_template(&mut self, category_id: usize, subcategory_id: usize, template: Option<SubcategoryTemplate>) {
        if subcategory_id == 0 {
            return;
        }
        let subcategory = self
            .categories
            .iter_mut()
            .find(|c| c.id == category_id)
            .and_then(|c| c.subcategories.iter_mut().find(|s| s.id == subcategory_id));
        if let Some(subcategory) = subcategory {
            subcategory.template = template.map(|t| t.key().to_string());
        }
    }

    /// Renomme une catégorie et remplace ses noms traduits. Un nom vide
    /// conserve l'ancien ; les traductions vides sont ignorées.
    pub fn update_category(&mut self, category_id: usize, name: String, translations: HashMap<Language, String>) {
        if let Some(category) = self.categories.iter_mut().find(|c| c.id == category_id) {
            apply_names(&mut category.name, &mut category.translations, name, translations);
        }
    }

    /// Même chose que `update_category` pour une sous-catégorie ("All",
    /// id 0, n'est pas modifiable : son nom suit la langue de l'interface).
    pub fn update_subcategory(
        &mut self,
        category_id: usize,
        subcategory_id: usize,
        name: String,
        translations: HashMap<Language, String>,
    ) {
        if subcategory_id == 0 {
            return;
        }
        if let Some(sub) = self
            .categories
            .iter_mut()
            .find(|c| c.id == category_id)
            .and_then(|c| c.subcategories.iter_mut().find(|s| s.id == subcategory_id))
        {
            apply_names(&mut sub.name, &mut sub.translations, name, translations);
        }
    }

    /// Supprime une catégorie entière, avec toutes ses sous-catégories
    /// (y compris "All", qui ne peut disparaître qu'avec sa catégorie parente).
    pub fn remove_category(&mut self, category_id: usize) {
        self.categories.retain(|c| c.id != category_id);
    }

    /// Supprime une sous-catégorie précise. Ne fait rien si on tente de
    /// supprimer "All" (id toujours 0) : elle ne peut être supprimée
    /// qu'en même temps que sa catégorie parente, via `remove_category`.
    pub fn remove_subcategory(&mut self, category_id: usize, subcategory_id: usize) {
        if subcategory_id == 0 {
            return;
        }
        if let Some(category) = self.categories.iter_mut().find(|c| c.id == category_id) {
            category.subcategories.retain(|s| s.id != subcategory_id);
        }
    }
}