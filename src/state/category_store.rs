use serde::{Deserialize, Serialize};
use crate::models::Category;

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
    pub fn add_subcategory(&mut self, category_id: usize, name: String) {
        if let Some(category) = self.categories.iter_mut().find(|c| c.id == category_id) {
            category.add_subcategory(name);
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