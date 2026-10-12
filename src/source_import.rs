use std::collections::HashMap;

use serde_json::Value;

use crate::json_path;
use crate::models::{ApiSource, ItemInfoField, ListPagination, ProgressNode, ProgressNodeKind, SourceListLevel};
use crate::net::{self, SearchError};

/// Plafond de pages par liste, au cas où une API ignorerait la pagination.
pub const MAX_PAGES: usize = 100;

/// Pause entre deux requêtes : certaines API publiques (Jikan...) limitent
/// le nombre d'appels par seconde.
#[cfg(not(target_arch = "wasm32"))]
const REQUEST_DELAY: std::time::Duration = std::time::Duration::from_millis(400);

/// Construit toute la progression décrite par `source.list_levels` à partir
/// du résultat de recherche `root`. Les nœuds sont créés niveau par niveau
/// (parcours en largeur, sans récursion async) avec des ids séquentiels.
/// `max_pages` limite le nombre de pages lues pour chaque liste (1 pour un
/// simple test).
pub async fn fetch_progress(source: &ApiSource, root: &Value, max_pages: usize) -> Result<Vec<ProgressNode>, SearchError> {
    let levels = &source.list_levels;
    let mut nodes: Vec<ProgressNode> = Vec::new();
    let mut parents: Vec<(Value, Option<usize>)> = vec![(root.clone(), None)];
    let mut has_requested = false;

    for (level_index, level) in levels.iter().enumerate() {
        let is_last = level_index + 1 == levels.len();
        let kind = node_kind(levels, level_index);
        let children_kind = if is_last { None } else { Some(node_kind(levels, level_index + 1)) };
        let mut next_parents = Vec::new();

        for (parent_json, parent_id) in parents {
            let elements = level_elements(level, &parent_json, root, max_pages, &mut has_requested).await?;
            for (position, element) in elements.into_iter().enumerate() {
                let id = nodes.len();
                nodes.push(build_node(level, &element, root, id, parent_id, kind, children_kind, position));
                if !is_last {
                    next_parents.push((element, Some(id)));
                }
            }
        }

        parents = next_parents;
    }

    Ok(nodes)
}

/// `Group` pour tous les niveaux sauf le dernier, qui prend `leaf_kind`
/// (Text par défaut si un `Group` s'y est glissé).
fn node_kind(levels: &[SourceListLevel], level_index: usize) -> ProgressNodeKind {
    if level_index + 1 < levels.len() {
        return ProgressNodeKind::Group;
    }
    match levels[level_index].leaf_kind {
        ProgressNodeKind::Card => ProgressNodeKind::Card,
        _ => ProgressNodeKind::Text,
    }
}

/// Les éléments JSON d'un niveau pour un parent donné : lus directement
/// dans le parent, ou récupérés via une (ou plusieurs, si paginée) requête.
async fn level_elements(
    level: &SourceListLevel,
    parent: &Value,
    root: &Value,
    max_pages: usize,
    has_requested: &mut bool,
) -> Result<Vec<Value>, SearchError> {
    if level.url_template.trim().is_empty() {
        return Ok(json_path::resolve_as_array(parent, &level.items_path).cloned().unwrap_or_default());
    }

    let mut elements = Vec::new();
    let mut previous_page: Option<Vec<Value>> = None;
    let mut full_page_len: Option<usize> = None;

    for page_index in 0..max_pages.max(1) {
        let vars = match level.pagination {
            ListPagination::None => Vec::new(),
            ListPagination::Page { first } => vec![("page", (first + page_index).to_string())],
            ListPagination::Offset { page_size } => vec![("offset", (page_index * page_size).to_string())],
        };
        let (url, _) = net::fill_template(&level.url_template, parent, root, &vars, true);

        if *has_requested {
            pause().await;
        }
        *has_requested = true;

        // Une erreur après la première page (ex: 404 au-delà de la
        // dernière) termine juste la pagination.
        let body = match net::fetch_json(&url).await {
            Ok(body) => body,
            Err(e) if page_index == 0 => return Err(e),
            Err(_) => break,
        };
        let page = net::results_array(&body, &level.items_path);

        // Une API qui ignore le paramètre de page renverrait sans fin la
        // même page.
        if page.is_empty() || previous_page.as_ref() == Some(&page) {
            break;
        }

        let page_len = page.len();
        elements.extend(page.iter().cloned());

        let expected_len = match level.pagination {
            ListPagination::None => break,
            ListPagination::Page { .. } => *full_page_len.get_or_insert(page_len),
            ListPagination::Offset { page_size } => page_size,
        };
        if page_len < expected_len {
            break;
        }
        previous_page = Some(page);
    }

    Ok(elements)
}

async fn pause() {
    #[cfg(not(target_arch = "wasm32"))]
    tokio::time::sleep(REQUEST_DELAY).await;
}

#[allow(clippy::too_many_arguments)]
fn build_node(
    level: &SourceListLevel,
    element: &Value,
    root: &Value,
    id: usize,
    parent_id: Option<usize>,
    kind: ProgressNodeKind,
    children_kind: Option<ProgressNodeKind>,
    position: usize,
) -> ProgressNode {
    let title = extract_text(&level.title_path, element, root)
        .unwrap_or_else(|| format!("{} {}", level.label, position + 1).trim().to_string());

    let image_url = if level.image_path.trim().is_empty() {
        None
    } else {
        extract_text(&level.image_path, element, root)
    };

    let fields = level
        .fields
        .iter()
        .filter_map(|field| {
            extract_text(&field.json_path, element, root).map(|value| ItemInfoField { label: field.label.clone(), value })
        })
        .collect();

    ProgressNode {
        id,
        parent_id,
        kind,
        children_kind,
        title,
        is_done: false,
        image_url,
        fields,
        redirect_links: Vec::new(),
    }
}

/// Résout `path_or_template` sur `element` : un chemin JSON simple, ou un
/// gabarit s'il contient `{` (dans ce cas il faut que tous ses marqueurs
/// existent). Renvoie `None` si rien d'exploitable n'est trouvé.
fn extract_text(path_or_template: &str, element: &Value, root: &Value) -> Option<String> {
    let path_or_template = path_or_template.trim();
    if path_or_template.is_empty() {
        return None;
    }

    let text = if path_or_template.contains('{') {
        let (text, all_found) = net::fill_template(path_or_template, element, root, &[], false);
        all_found.then_some(text)?
    } else {
        json_path::resolve(element, path_or_template).and_then(net::value_text)?
    };

    let text = text.trim().to_string();
    (!text.is_empty()).then_some(text)
}

/// Fusionne une progression fraîchement réimportée avec l'ancienne : les
/// nœuds sont associés par leur chaîne de titres (ex: "Saison 1 / Épisode 3")
/// pour conserver ce qui a été coché et les liens ajoutés. Les anciens nœuds
/// absents de l'import (ajoutés à la main...) sont gardés à la fin.
pub fn merge_progress(old: &[ProgressNode], fresh: Vec<ProgressNode>) -> Vec<ProgressNode> {
    let old_keys = node_keys(old);
    let fresh_keys = node_keys(&fresh);
    let old_by_key: HashMap<&str, &ProgressNode> = old.iter().map(|n| (old_keys[&n.id].as_str(), n)).collect();
    let fresh_id_by_key: HashMap<&str, usize> = fresh.iter().map(|n| (fresh_keys[&n.id].as_str(), n.id)).collect();

    let mut merged: Vec<ProgressNode> = fresh
        .iter()
        .map(|node| {
            let mut node = node.clone();
            if let Some(previous) = old_by_key.get(fresh_keys[&node.id].as_str()) {
                node.is_done = previous.is_done;
                node.redirect_links = previous.redirect_links.clone();
            }
            node
        })
        .collect();

    // Les anciens nœuds conservés reçoivent un id décalé après les nouveaux.
    // Leur parent est soit un nœud réimporté (même clé), soit lui-même un
    // ancien nœud conservé (donc décalé pareil).
    let offset = fresh.iter().map(|n| n.id + 1).max().unwrap_or(0);
    let new_id_of_old = |old_id: usize| -> usize {
        old_keys
            .get(&old_id)
            .and_then(|key| fresh_id_by_key.get(key.as_str()).copied())
            .unwrap_or(offset + old_id)
    };

    for node in old {
        if fresh_id_by_key.contains_key(old_keys[&node.id].as_str()) {
            continue;
        }
        let mut kept = node.clone();
        kept.id = offset + node.id;
        kept.parent_id = node.parent_id.map(new_id_of_old);
        merged.push(kept);
    }

    merged
}

/// Clé de fusion de chaque nœud (par id) : sa chaîne de titres, suivie de
/// son rang parmi les nœuds ayant la même chaîne (deux épisodes "TBA" dans
/// la même saison restent distincts).
fn node_keys(nodes: &[ProgressNode]) -> HashMap<usize, String> {
    let mut occurrences: HashMap<String, usize> = HashMap::new();
    nodes
        .iter()
        .map(|node| {
            let chain = title_chain(nodes, node);
            let rank = occurrences.entry(chain.clone()).or_insert(0);
            *rank += 1;
            (node.id, format!("{chain}\u{1e}{rank}"))
        })
        .collect()
}

/// Titres du nœud et de tous ses ancêtres, de la racine au nœud.
fn title_chain(nodes: &[ProgressNode], node: &ProgressNode) -> String {
    let mut titles = vec![node.title.as_str()];
    let mut current = node.parent_id;
    // Garde-fou contre un cycle de `parent_id` dans un fichier corrompu.
    while let Some(parent_id) = current.filter(|_| titles.len() <= nodes.len()) {
        match nodes.iter().find(|n| n.id == parent_id) {
            Some(parent) => {
                titles.push(parent.title.as_str());
                current = parent.parent_id;
            }
            None => break,
        }
    }
    titles.reverse();
    titles.join("\u{1f}")
}
