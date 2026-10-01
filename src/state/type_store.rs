use serde::{Deserialize, Serialize};

use crate::models::ItemType;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TypeStore {
    pub types: Vec<ItemType>,
    next_id: usize,
}

impl TypeStore {
    /// Ajoute un type pour une catégorie donnée. Si un type du même nom
    /// existe déjà DANS CETTE CATÉGORIE, on renvoie son id existant plutôt
    /// que d'en créer un doublon (le même nom reste possible dans une
    /// autre catégorie, puisque les types sont propres à chaque catégorie).
    pub fn add_type(&mut self, category_id: usize, name: String) -> usize {
        let trimmed = name.trim();

        if let Some(existing) = self.types.iter().find(|t| {
            t.category_id == category_id && t.name.eq_ignore_ascii_case(trimmed)
        }) {
            return existing.id;
        }

        let id = self.next_id;
        self.next_id += 1;
        self.types.push(ItemType {
            id,
            category_id,
            name: trimmed.to_string(),
        });
        id
    }

    pub fn remove_type(&mut self, id: usize) {
        self.types.retain(|t| t.id != id);
    }

    /// Supprime tous les types d'une catégorie (utilisé quand la catégorie
    /// elle-même est supprimée).
    pub fn remove_types_by_category(&mut self, category_id: usize) {
        self.types.retain(|t| t.category_id != category_id);
    }
}