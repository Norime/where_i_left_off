use serde::{Deserialize, Serialize};

use crate::models::{entry_placement, Item, Universe, UniverseEntry, UniverseOrder, UniverseOrderKind};

/// Univers du compte (voir `models::Universe`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct UniverseStore {
    pub universes: Vec<Universe>,
    #[serde(default)]
    next_universe_id: usize,
}

impl UniverseStore {
    pub fn universe(&self, universe_id: usize) -> Option<&Universe> {
        self.universes.iter().find(|u| u.id == universe_id)
    }

    fn universe_mut(&mut self, universe_id: usize) -> Option<&mut Universe> {
        self.universes.iter_mut().find(|u| u.id == universe_id)
    }

    /// Crée un univers avec deux ordres : sortie (automatique) et un ordre
    /// libre, nommés dans la langue active. Retourne son id.
    pub fn add_universe(&mut self, name: String, release_name: String, manual_name: String) -> usize {
        let id = self.next_universe_id.max(self.universes.iter().map(|u| u.id + 1).max().unwrap_or(0));
        self.next_universe_id = id + 1;
        self.universes.push(Universe::new(id, name.trim().to_string(), release_name, manual_name));
        id
    }

    pub fn rename_universe(&mut self, universe_id: usize, name: String) {
        let name = name.trim().to_string();
        if let (false, Some(universe)) = (name.is_empty(), self.universe_mut(universe_id)) {
            universe.name = name;
        }
    }

    pub fn remove_universe(&mut self, universe_id: usize) {
        self.universes.retain(|u| u.id != universe_id);
    }

    /// Ajoute une œuvre (un placement précis) à l'univers, à la fin des
    /// ordres libres. Ne fait rien si elle y est déjà.
    pub fn add_entry(&mut self, universe_id: usize, item_id: usize, category_id: usize, subcategory_id: usize) {
        let Some(universe) = self.universe_mut(universe_id) else { return };
        if universe.contains(item_id, category_id, subcategory_id) {
            return;
        }
        let id = universe.entries.iter().map(|e| e.id + 1).max().unwrap_or(0);
        universe.entries.push(UniverseEntry { id, item_id, category_id, subcategory_id });
        for order in universe.orders.iter_mut() {
            if let UniverseOrderKind::Manual(ids) = &mut order.kind {
                ids.push(id);
            }
        }
    }

    pub fn remove_entry(&mut self, universe_id: usize, entry_id: usize) {
        let Some(universe) = self.universe_mut(universe_id) else { return };
        universe.entries.retain(|e| e.id != entry_id);
        for order in universe.orders.iter_mut() {
            if let UniverseOrderKind::Manual(ids) = &mut order.kind {
                ids.retain(|id| *id != entry_id);
            }
        }
    }

    /// Ajoute un ordre (libre ou de sortie) et l'affiche. Un ordre libre
    /// part de l'ordre affiché jusque-là.
    pub fn add_order(&mut self, universe_id: usize, name: String, manual: bool, items: &[Item]) {
        let Some(universe) = self.universe_mut(universe_id) else { return };
        let name = name.trim().to_string();
        if name.is_empty() {
            return;
        }
        let kind = if manual {
            let start = universe
                .current_order()
                .map(|order| universe.ordered_entries(order, items).iter().map(|e| e.id).collect())
                .unwrap_or_default();
            UniverseOrderKind::Manual(start)
        } else {
            UniverseOrderKind::Release
        };
        let id = universe.orders.iter().map(|o| o.id + 1).max().unwrap_or(0);
        universe.orders.push(UniverseOrder { id, name, kind });
        universe.selected_order = id;
    }

    pub fn rename_order(&mut self, universe_id: usize, order_id: usize, name: String) {
        let name = name.trim().to_string();
        let Some(universe) = self.universe_mut(universe_id) else { return };
        if let (false, Some(order)) = (name.is_empty(), universe.orders.iter_mut().find(|o| o.id == order_id)) {
            order.name = name;
        }
    }

    /// Supprime un ordre ; le dernier ordre restant ne peut pas l'être.
    pub fn remove_order(&mut self, universe_id: usize, order_id: usize) {
        let Some(universe) = self.universe_mut(universe_id) else { return };
        if universe.orders.len() > 1 {
            universe.orders.retain(|o| o.id != order_id);
        }
    }

    pub fn select_order(&mut self, universe_id: usize, order_id: usize) {
        if let Some(universe) = self.universe_mut(universe_id) {
            universe.selected_order = order_id;
        }
    }

    /// Monte (`-1`) ou descend (`+1`) une entrée dans un ordre libre.
    pub fn move_entry(&mut self, universe_id: usize, order_id: usize, entry_id: usize, offset: isize, items: &[Item]) {
        let Some(universe) = self.universe_mut(universe_id) else { return };
        let Some(order) = universe.orders.iter().find(|o| o.id == order_id).cloned() else { return };
        if !matches!(order.kind, UniverseOrderKind::Manual(_)) {
            return;
        }
        // Repart de l'ordre affiché, qui inclut les entrées absentes de la liste.
        let mut ids: Vec<usize> = universe.ordered_entries(&order, items).iter().map(|e| e.id).collect();
        let Some(index) = ids.iter().position(|id| *id == entry_id) else { return };
        let target = index as isize + offset;
        if target < 0 || target as usize >= ids.len() {
            return;
        }
        ids.swap(index, target as usize);
        if let Some(order) = universe.orders.iter_mut().find(|o| o.id == order_id) {
            order.kind = UniverseOrderKind::Manual(ids);
        }
    }

    /// Une entrée pointe vers une fiche ou un placement qui n'existe plus.
    pub fn has_missing_entries(&self, items: &[Item]) -> bool {
        self.universes.iter().flat_map(|u| u.entries.iter()).any(|e| entry_placement(e, items).is_none())
    }

    /// Retire les entrées dont la fiche ou le placement n'existe plus
    /// (fiche supprimée, catégorie retirée…). Vrai si quelque chose a changé.
    pub fn remove_missing_entries(&mut self, items: &[Item]) -> bool {
        let mut changed = false;
        for universe in self.universes.iter_mut() {
            let missing: Vec<usize> = universe
                .entries
                .iter()
                .filter(|e| entry_placement(e, items).is_none())
                .map(|e| e.id)
                .collect();
            if missing.is_empty() {
                continue;
            }
            changed = true;
            universe.entries.retain(|e| !missing.contains(&e.id));
            for order in universe.orders.iter_mut() {
                if let UniverseOrderKind::Manual(ids) = &mut order.kind {
                    ids.retain(|id| !missing.contains(id));
                }
            }
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ItemOrientation, ItemPlacement};

    fn item(id: usize) -> Item {
        Item {
            id,
            title: format!("{id}"),
            description: String::new(),
            placements: vec![ItemPlacement::new(1, 1)],
            orientation: ItemOrientation::Vertical,
            image_url: None,
            extra_info: Vec::new(),
        }
    }

    #[test]
    fn entries_follow_items_and_manual_moves() {
        let mut items = vec![item(0), item(1), item(2)];
        let mut store = UniverseStore::default();
        let id = store.add_universe("MCU".into(), "Sortie".into(), "Chrono".into());
        for item in items.iter() {
            store.add_entry(id, item.id, 1, 1);
        }
        store.add_entry(id, 0, 1, 1);
        assert_eq!(store.universe(id).unwrap().entries.len(), 3);

        store.move_entry(id, 1, 2, -1, &items);
        let universe = store.universe(id).unwrap();
        let manual = universe.orders[1].clone();
        let order: Vec<usize> = universe.ordered_entries(&manual, &items).iter().map(|e| e.item_id).collect();
        assert_eq!(order, vec![0, 2, 1]);

        items.remove(1);
        assert!(store.has_missing_entries(&items));
        assert!(store.remove_missing_entries(&items));
        assert!(!store.has_missing_entries(&items));
        let universe = store.universe(id).unwrap();
        assert_eq!(universe.orders[1].kind, UniverseOrderKind::Manual(vec![0, 2]));
    }
}
