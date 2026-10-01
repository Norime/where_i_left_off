use serde::{Deserialize, Serialize};
use crate::models::{Item, ItemInfoField, ProgressLink, ProgressNode, ProgressNodeKind, ItemOrientation, ItemPlacement, ItemStatus, WorkStatus, ItemTab};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct ItemStore {
    pub items: Vec<Item>,
    next_item_id: usize,
}

impl ItemStore {
    pub fn add_item(
        &mut self,
        title: String,
        description: String,
        initial_placement: ItemPlacement,
        orientation: ItemOrientation,
        image_url: Option<String>,
        extra_info: Vec<ItemInfoField>,
    ) -> usize {
        let id = self.next_item_id;
        self.next_item_id += 1;
        self.items.push(Item {
            id,
            title,
            description,
            placements: vec![initial_placement],
            orientation,
            image_url,
            extra_info,
        });
        id
    }

    pub fn update_item(
        &mut self,
        id: usize,
        title: String,
        description: String,
        orientation: ItemOrientation,
        image_url: Option<String>,
        extra_info: Vec<ItemInfoField>,
    ) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.title = title;
            item.description = description;
            item.orientation = orientation;
            item.image_url = image_url;
            item.extra_info = extra_info;
        }
    }

    /// Petit utilitaire interne : retrouve le placement précis (catégorie +
    /// sous-catégorie) d'un item, pour éviter de répéter cette recherche
    /// dans chaque méthode de type/progression ci-dessous.
    fn find_placement_mut(&mut self, item_id: usize, category_id: usize, subcategory_id: usize) -> Option<&mut ItemPlacement> {
        let item = self.items.iter_mut().find(|i| i.id == item_id)?;
        item.placements.iter_mut().find(|p| p.category_id == category_id && p.subcategory_id == subcategory_id)
    }

    /// Recalcule automatiquement le statut d'un placement à partir de sa
    /// progression : Nouveau si rien n'est fait, En cours si c'est partiel,
    /// Terminé si tout est fait. Ne touche à rien si le placement n'a
    /// aucun nœud de progression cochable (le statut reste alors manuel,
    /// piloté par `set_placement_status`).
    fn recompute_placement_status(&mut self, item_id: usize, category_id: usize, subcategory_id: usize) {
        if let Some(placement) = self.find_placement_mut(item_id, category_id, subcategory_id) {
            let total = placement.progress.iter().filter(|n| n.kind != ProgressNodeKind::Group).count();
            if total > 0 {
                let done = placement.progress.iter().filter(|n| n.kind != ProgressNodeKind::Group && n.is_done).count();
                placement.status = if done == 0 {
                    ItemStatus::New
                } else if done < total {
                    ItemStatus::InProgress
                } else {
                    ItemStatus::Done
                };
            }
        }
    }

    pub fn set_placement_types(&mut self, item_id: usize, category_id: usize, subcategory_id: usize, new_type_ids: Vec<usize>) {
        if let Some(placement) = self.find_placement_mut(item_id, category_id, subcategory_id) {
            placement.type_ids = new_type_ids;
        }
    }

    /// Retire les ids de types supprimés de TOUS les placements de TOUS les
    /// items (un type étant propre à une catégorie, seuls les placements de
    /// cette catégorie sont concernés en pratique).
    pub fn remove_type_references(&mut self, removed_type_ids: &[usize]) {
        for item in self.items.iter_mut() {
            for placement in item.placements.iter_mut() {
                placement.type_ids.retain(|id| !removed_type_ids.contains(id));
            }
        }
    }

    /// Ajoute un placement supplémentaire (statut "Nouveau" par défaut, pas
    /// de type ni de progression : à renseigner ensuite). Ne fait rien si
    /// ce placement existe déjà.
    pub fn add_placement(&mut self, item_id: usize, category_id: usize, subcategory_id: usize) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == item_id) {
            let already_there = item.placements.iter()
                .any(|p| p.category_id == category_id && p.subcategory_id == subcategory_id);
            if !already_there {
                item.placements.push(ItemPlacement {
                    category_id,
                    subcategory_id,
                    status: ItemStatus::New,
                    type_ids: Vec::new(),
                    progress: Vec::new(),
                    work_status: WorkStatus::default(),
                    tabs: Vec::new(),
                });
            }
        }
    }

    /// Change le statut d'un placement précis d'un élément.
    pub fn set_placement_status(&mut self, item_id: usize, category_id: usize, subcategory_id: usize, status: ItemStatus) {
        if let Some(placement) = self.find_placement_mut(item_id, category_id, subcategory_id) {
            placement.status = status;
        }
    }

    /// Statut de L'ŒUVRE elle-même (en cours / terminé / non fini) pour CE
    /// placement précis — un même item peut être "non fini" côté anime et
    /// "en cours" côté manga, ce sont deux placements distincts.
    pub fn set_placement_work_status(&mut self, item_id: usize, category_id: usize, subcategory_id: usize, status: WorkStatus) {
        if let Some(placement) = self.find_placement_mut(item_id, category_id, subcategory_id) {
            placement.work_status = status;
        }
    }

    /// Retire un placement précis d'un élément. Refuse si c'est son dernier
    /// placement.
    pub fn remove_placement(&mut self, item_id: usize, category_id: usize, subcategory_id: usize) -> Result<(), String> {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == item_id) {
            if item.placements.len() <= 1 {
                return Err("Un élément doit rester dans au moins une catégorie.".to_string());
            }
            item.placements.retain(|p| !(p.category_id == category_id && p.subcategory_id == subcategory_id));
        }
        Ok(())
    }

    /// Supprime un élément par son id.
    pub fn remove_item(&mut self, item_id: usize) {
        self.items.retain(|item| item.id != item_id);
    }

    /// Retire tous les placements pointant vers une catégorie donnée
    /// (cascade : ça supprime au passage leurs types et leur progression,
    /// puisqu'ils vivent maintenant sur le placement), puis supprime les
    /// éléments qui n'ont plus aucun placement.
    pub fn remove_placements_by_category(&mut self, category_id: usize) {
        for item in self.items.iter_mut() {
            item.placements.retain(|p| p.category_id != category_id);
        }
        self.items.retain(|item| !item.placements.is_empty());
    }

    /// Retire le placement pointant vers une sous-catégorie précise
    /// (cascade), puis supprime les éléments qui n'ont plus aucun placement.
    pub fn remove_placements_by_subcategory(&mut self, category_id: usize, subcategory_id: usize) {
        for item in self.items.iter_mut() {
            item.placements.retain(|p| !(p.category_id == category_id && p.subcategory_id == subcategory_id));
        }
        self.items.retain(|item| !item.placements.is_empty());
    }

    // --- Progression (propre à chaque placement) ---------------------

    /// Ajoute un nœud de progression au placement (category_id,
    /// subcategory_id) de cet item. `parent_id` est l'id d'un autre nœud
    /// DU MÊME PLACEMENT si c'est un sous-élément, ou `None` si c'est un
    /// élément racine. L'id est généré à partir du max existant dans CE
    /// placement (pas besoin d'un compteur global).
    pub fn add_progress_node(
        &mut self,
        item_id: usize,
        category_id: usize,
        subcategory_id: usize,
        parent_id: Option<usize>,
        kind: ProgressNodeKind,
        children_kind: Option<ProgressNodeKind>,
        title: String,
    ) -> Option<usize> {
        let placement = self.find_placement_mut(item_id, category_id, subcategory_id)?;
        let next_id = placement.progress.iter().map(|n| n.id).max().map(|m| m + 1).unwrap_or(0);
        placement.progress.push(ProgressNode {
            id: next_id,
            parent_id,
            kind,
            children_kind,
            title,
            is_done: false,
            image_url: None,
            fields: Vec::new(),
            redirect_links: Vec::new(),
        });
        self.recompute_placement_status(item_id, category_id, subcategory_id);
        Some(next_id)
    }

    /// Supprime un nœud ET tous ses descendants, à n'importe quelle
    /// profondeur (supprimer une saison supprime ses épisodes, supprimer
    /// un coffret supprime ses saisons ET leurs épisodes).
    pub fn remove_progress_node(&mut self, item_id: usize, category_id: usize, subcategory_id: usize, node_id: usize) {
        if let Some(placement) = self.find_placement_mut(item_id, category_id, subcategory_id) {
            let mut to_remove = vec![node_id];
            let mut index = 0;
            while index < to_remove.len() {
                let current = to_remove[index];
                for node in placement.progress.iter() {
                    if node.parent_id == Some(current) && !to_remove.contains(&node.id) {
                        to_remove.push(node.id);
                    }
                }
                index += 1;
            }
            placement.progress.retain(|n| !to_remove.contains(&n.id));
        }
        self.recompute_placement_status(item_id, category_id, subcategory_id);
    }

    pub fn set_progress_node_done(&mut self, item_id: usize, category_id: usize, subcategory_id: usize, node_id: usize, is_done: bool) {
        if let Some(placement) = self.find_placement_mut(item_id, category_id, subcategory_id) {
            if let Some(node) = placement.progress.iter_mut().find(|n| n.id == node_id) {
                node.is_done = is_done;
            }
        }
        self.recompute_placement_status(item_id, category_id, subcategory_id);
    }

    /// Coche/décoche un nœud ET tous ses descendants (à n'importe quelle
    /// profondeur) d'un coup — utilisé par la case à cocher "en cascade"
    /// sur les listes (Group).
    pub fn set_progress_subtree_done(&mut self, item_id: usize, category_id: usize, subcategory_id: usize, node_id: usize, is_done: bool) {
        if let Some(placement) = self.find_placement_mut(item_id, category_id, subcategory_id) {
            let mut to_update = vec![node_id];
            let mut index = 0;
            while index < to_update.len() {
                let current = to_update[index];
                for node in placement.progress.iter() {
                    if node.parent_id == Some(current) && !to_update.contains(&node.id) {
                        to_update.push(node.id);
                    }
                }
                index += 1;
            }
            for node in placement.progress.iter_mut() {
                if node.kind != ProgressNodeKind::Group && to_update.contains(&node.id) {
                    node.is_done = is_done;
                }
            }
        }
        self.recompute_placement_status(item_id, category_id, subcategory_id);
    }

    pub fn set_progress_node_title(&mut self, item_id: usize, category_id: usize, subcategory_id: usize, node_id: usize, title: String) {
        if let Some(placement) = self.find_placement_mut(item_id, category_id, subcategory_id) {
            if let Some(node) = placement.progress.iter_mut().find(|n| n.id == node_id) {
                node.title = title;
            }
        }
    }

    pub fn set_progress_node_fields(&mut self, item_id: usize, category_id: usize, subcategory_id: usize, node_id: usize, fields: Vec<ItemInfoField>) {
        if let Some(placement) = self.find_placement_mut(item_id, category_id, subcategory_id) {
            if let Some(node) = placement.progress.iter_mut().find(|n| n.id == node_id) {
                node.fields = fields;
            }
        }
    }

    pub fn set_progress_node_image(&mut self, item_id: usize, category_id: usize, subcategory_id: usize, node_id: usize, image_url: Option<String>) {
        if let Some(placement) = self.find_placement_mut(item_id, category_id, subcategory_id) {
            if let Some(node) = placement.progress.iter_mut().find(|n| n.id == node_id) {
                node.image_url = image_url;
            }
        }
    }

    pub fn set_progress_node_links(&mut self, item_id: usize, category_id: usize, subcategory_id: usize, node_id: usize, links: Vec<ProgressLink>) {
        if let Some(placement) = self.find_placement_mut(item_id, category_id, subcategory_id) {
            if let Some(node) = placement.progress.iter_mut().find(|n| n.id == node_id) {
                node.redirect_links = links;
            }
        }
    }

    /// Remplace toute la progression d'un placement d'un coup (utile pour
    /// un import en masse depuis une API).
    pub fn set_placement_progress(&mut self, item_id: usize, category_id: usize, subcategory_id: usize, progress: Vec<ProgressNode>) {
        if let Some(placement) = self.find_placement_mut(item_id, category_id, subcategory_id) {
            placement.progress = progress;
        }
        self.recompute_placement_status(item_id, category_id, subcategory_id);
    }

    // --- Onglets personnalisés (propres à chaque placement) -----------

    /// Ajoute un onglet personnalisé (zone vide pour l'instant) au
    /// placement (category_id, subcategory_id) de cet item. L'id est
    /// généré à partir du max existant DANS CE PLACEMENT.
    pub fn add_tab(&mut self, item_id: usize, category_id: usize, subcategory_id: usize, title: String) -> Option<usize> {
        let placement = self.find_placement_mut(item_id, category_id, subcategory_id)?;
        let next_id = placement.tabs.iter().map(|t| t.id).max().map(|m| m + 1).unwrap_or(0);
        placement.tabs.push(ItemTab { id: next_id, title });
        Some(next_id)
    }

    /// Supprime un onglet personnalisé. L'onglet "Progression" n'est pas
    /// stocké ici (il est toujours présent), donc rien à faire pour lui.
    pub fn remove_tab(&mut self, item_id: usize, category_id: usize, subcategory_id: usize, tab_id: usize) {
        if let Some(placement) = self.find_placement_mut(item_id, category_id, subcategory_id) {
            placement.tabs.retain(|t| t.id != tab_id);
        }
    }
}