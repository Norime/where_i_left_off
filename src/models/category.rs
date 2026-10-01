use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Subcategory {
    pub id: usize,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Category {
    pub id: usize,
    pub name: String,
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
            subcategories: Vec::new(),
            next_subcategory_id: 0,
        };
        category.add_subcategory("All".to_string());
        category
    }

    /// Ajoute une nouvelle sous-catégorie, avec un id unique généré
    /// via le compteur interne `next_subcategory_id`.
    pub fn add_subcategory(&mut self, name: String) {
        let id = self.next_subcategory_id;
        self.next_subcategory_id += 1;
        self.subcategories.push(Subcategory { id, name });
    }
}