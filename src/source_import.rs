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

pub async fn pause() {
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
        is_new: false,
        new_since: None,
        resume_at: None,
    }
}

/// Résout `path_or_template` sur `element` : un chemin JSON simple, ou un
/// gabarit s'il contient `{` (dans ce cas il faut que tous ses marqueurs
/// existent). Renvoie `None` si rien d'exploitable n'est trouvé.
///
/// Un gabarit accepte des alternatives séparées par `|` hors des accolades :
/// `"{titre}. {tome}|{titre}"` se rabat sur le titre seul s'il n'y a pas
/// de tome.
pub fn extract_text(path_or_template: &str, element: &Value, root: &Value) -> Option<String> {
    let path_or_template = path_or_template.trim();
    if path_or_template.contains('{') {
        return template_alternatives(path_or_template)
            .into_iter()
            .find_map(|alternative| extract_one(alternative, element, root));
    }
    extract_one(path_or_template, element, root)
}

/// Découpe `template` sur les `|` qui ne sont pas entre accolades (ceux
/// entre accolades sont des alternatives de chemin, gérées par `json_path`).
fn template_alternatives(template: &str) -> Vec<&str> {
    let mut alternatives = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    for (index, character) in template.char_indices() {
        match character {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            '|' if depth == 0 => {
                alternatives.push(&template[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    alternatives.push(&template[start..]);
    alternatives
}

fn extract_one(path_or_template: &str, element: &Value, root: &Value) -> Option<String> {
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
///
/// `mark_new` : les nœuds frais sans équivalent dans `old` sont marqués
/// `is_new` (jamais si `old` est vide, sinon tout serait "nouveau"), avec
/// `now` comme date de détection.
pub fn merge_progress(old: &[ProgressNode], fresh: Vec<ProgressNode>, mark_new: bool, now: u64) -> Vec<ProgressNode> {
    let mark_new = mark_new && !old.is_empty();
    let old_keys = node_keys(old);
    let fresh_keys = node_keys(&fresh);
    let old_by_key: HashMap<&str, &ProgressNode> = old.iter().map(|n| (old_keys[&n.id].as_str(), n)).collect();
    let fresh_id_by_key: HashMap<&str, usize> = fresh.iter().map(|n| (fresh_keys[&n.id].as_str(), n.id)).collect();

    let mut merged: Vec<ProgressNode> = fresh
        .iter()
        .map(|node| {
            let mut node = node.clone();
            match old_by_key.get(fresh_keys[&node.id].as_str()) {
                Some(previous) => {
                    node.is_done = previous.is_done;
                    node.redirect_links = previous.redirect_links.clone();
                    node.resume_at = previous.resume_at.clone();
                    node.is_new = previous.is_new;
                    // Nouveauté d'avant l'ajout de `new_since` : on la date
                    // d'aujourd'hui pour qu'elle finisse par expirer.
                    node.new_since = previous.new_since.or(previous.is_new.then_some(now));
                }
                None => {
                    node.is_new = mark_new;
                    node.new_since = mark_new.then_some(now);
                }
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

/// Les éléments cochables (pas les `Group`) nouveaux et pas encore vus.
pub fn new_leaves(nodes: &[ProgressNode]) -> Vec<&ProgressNode> {
    nodes
        .iter()
        .filter(|n| n.kind != ProgressNodeKind::Group && n.is_new && !n.is_done)
        .collect()
}

/// Durée pendant laquelle un élément détecté reste dans les nouveautés.
pub const NEW_RELEASE_WINDOW_SECS: u64 = 14 * 24 * 3600;

/// Heure actuelle en secondes Unix (fonctionne aussi en wasm).
pub fn now_secs() -> u64 {
    web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// `new_leaves` limité aux éléments détectés depuis moins de
/// `NEW_RELEASE_WINDOW_SECS`. Sans date de détection (données anciennes pas
/// encore revérifiées), l'élément reste affiché.
pub fn recent_new_leaves(nodes: &[ProgressNode], now: u64) -> Vec<&ProgressNode> {
    new_leaves(nodes)
        .into_iter()
        .filter(|n| n.new_since.is_none_or(|since| now.saturating_sub(since) < NEW_RELEASE_WINDOW_SECS))
        .collect()
}

/// Date de sortie fournie par la source, telle quelle : le premier champ
/// dont le label évoque une date ("Date de sortie", "Release date"...).
pub fn release_date(node: &ProgressNode) -> Option<&str> {
    node.fields
        .iter()
        .find(|f| {
            let label = f.label.to_lowercase();
            ["date", "sortie", "release", "air"].iter().any(|word| label.contains(word))
        })
        .map(|f| f.value.as_str())
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

#[cfg(test)]
mod tests {
    use super::{extract_text, merge_progress};
    use crate::models::{ProgressNode, ProgressNodeKind};
    use serde_json::json;

    fn episode(id: usize, title: &str) -> ProgressNode {
        ProgressNode {
            id,
            parent_id: None,
            kind: ProgressNodeKind::Card,
            children_kind: None,
            title: title.to_string(),
            is_done: false,
            image_url: None,
            fields: Vec::new(),
            redirect_links: Vec::new(),
            is_new: false,
            new_since: None,
            resume_at: None,
        }
    }

    #[test]
    fn reimport_keeps_resume_position() {
        let mut old = vec![episode(0, "Ép. 1"), episode(1, "Ép. 2")];
        old[1].resume_at = Some("34:12".to_string());
        let fresh = vec![episode(0, "Ép. 1"), episode(1, "Ép. 2"), episode(2, "Ép. 3")];
        let merged = merge_progress(&old, fresh, true, 0);
        let second = merged.iter().find(|n| n.title == "Ép. 2").unwrap();
        assert_eq!(second.resume_at.as_deref(), Some("34:12"));
    }

    #[test]
    fn template_alternatives_fall_back_when_a_marker_is_missing() {
        let with_volume = json!({ "title": "Frieren", "volume": "Vol. 6" });
        let without_volume = json!({ "title": "Frieren" });
        let template = "{title}. {volume}|{title}";
        assert_eq!(extract_text(template, &with_volume, &with_volume).as_deref(), Some("Frieren. Vol. 6"));
        assert_eq!(extract_text(template, &without_volume, &without_volume).as_deref(), Some("Frieren"));
        assert_eq!(extract_text("{volume}|title", &without_volume, &without_volume).as_deref(), Some("Frieren"));
    }
}
