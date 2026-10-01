use serde::{Deserialize, Serialize};

use crate::models::{ApiSource, SourceScope};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SourceStore {
    pub sources: Vec<ApiSource>,
    next_id: usize,
}

impl SourceStore {
    /// Les sources reliées à une sous-catégorie précise.
    pub fn sources_for_subcategory(&self, category_id: usize, subcategory_id: usize) -> Vec<ApiSource> {
        let scope = SourceScope { category_id, subcategory_id };
        self.sources
            .iter()
            .filter(|s| s.linked_scopes.contains(&scope))
            .cloned()
            .collect()
    }

    /// Crée une nouvelle source, déjà reliée au scope donné.
    pub fn add_source(&mut self, mut source: ApiSource) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        source.id = id;
        self.sources.push(source);
        id
    }

    /// Modifie les infos d'une source existante (nom, chemins JSON...)
    /// sans toucher à ses `linked_scopes`.
    pub fn update_source(&mut self, updated: ApiSource) {
        if let Some(existing) = self.sources.iter_mut().find(|s| s.id == updated.id) {
            *existing = updated;
        }
    }

    /// Relie une source existante à une sous-catégorie de plus (ne fait
    /// rien si elle y est déjà reliée).
    pub fn link_source(&mut self, source_id: usize, category_id: usize, subcategory_id: usize) {
        if let Some(source) = self.sources.iter_mut().find(|s| s.id == source_id) {
            let scope = SourceScope { category_id, subcategory_id };
            if !source.linked_scopes.contains(&scope) {
                source.linked_scopes.push(scope);
            }
        }
    }

    /// Délie une source d'une sous-catégorie précise. Si elle n'est plus
    /// reliée à rien après ça, elle est supprimée entièrement.
    pub fn unlink_source(&mut self, source_id: usize, category_id: usize, subcategory_id: usize) {
        let scope = SourceScope { category_id, subcategory_id };
        if let Some(source) = self.sources.iter_mut().find(|s| s.id == source_id) {
            source.linked_scopes.retain(|s| *s != scope);
        }
        self.sources.retain(|s| !s.linked_scopes.is_empty());
    }

    /// Retire tous les liens vers une catégorie (cascade de suppression de
    /// catégorie) ; supprime les sources qui se retrouvent sans lien.
    pub fn remove_scopes_by_category(&mut self, category_id: usize) {
        for source in self.sources.iter_mut() {
            source.linked_scopes.retain(|scope| scope.category_id != category_id);
        }
        self.sources.retain(|s| !s.linked_scopes.is_empty());
    }

    /// Retire tous les liens vers une sous-catégorie précise (cascade de
    /// suppression de sous-catégorie) ; supprime les sources qui se
    /// retrouvent sans lien.
    pub fn remove_scopes_by_subcategory(&mut self, category_id: usize, subcategory_id: usize) {
        for source in self.sources.iter_mut() {
            source
                .linked_scopes
                .retain(|scope| !(scope.category_id == category_id && scope.subcategory_id == subcategory_id));
        }
        self.sources.retain(|s| !s.linked_scopes.is_empty());
    }
}