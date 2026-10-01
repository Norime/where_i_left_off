use serde_json::Value;

/// Résout un chemin simple façon "a.b.c" ou "a[0].b" à l'intérieur d'une
/// valeur JSON (`serde_json::Value`). Retourne `None` si un des segments
/// n'existe pas ou n'a pas le bon type.
///
/// Exemples de chemins acceptés :
///   "title"                  -> value["title"]
///   "attributes.title"       -> value["attributes"]["title"]
///   "images.jpg.image_url"   -> value["images"]["jpg"]["image_url"]
///   "genres[0].name"         -> value["genres"][0]["name"]
///
/// Un chemin vide ("") renvoie la valeur elle-même (utile quand la donnée
/// est directement à la racine, par exemple pour `results_path` quand la
/// réponse EST déjà le tableau de résultats).
pub fn resolve<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let path = path.trim();
    if path.is_empty() {
        return Some(value);
    }

    let mut current = value;
    for raw_segment in path.split('.') {
        let (field_part, index_parts) = split_indices(raw_segment);

        if !field_part.is_empty() {
            current = current.as_object()?.get(field_part)?;
        }

        for idx in index_parts {
            current = current.as_array()?.get(idx)?;
        }
    }

    Some(current)
}

/// Sépare "genres[0][1]" en ("genres", [0, 1]). "[0]" seul donne ("", [0]).
fn split_indices(segment: &str) -> (&str, Vec<usize>) {
    let bracket_pos = segment.find('[');
    let (field, mut rest) = match bracket_pos {
        Some(pos) => (&segment[..pos], &segment[pos..]),
        None => (segment, ""),
    };

    let mut indices = Vec::new();
    while let Some(start) = rest.find('[') {
        if let Some(end) = rest[start..].find(']') {
            let end = start + end;
            if let Ok(idx) = rest[start + 1..end].parse::<usize>() {
                indices.push(idx);
            }
            rest = &rest[end + 1..];
        } else {
            break;
        }
    }

    (field, indices)
}

/// Variante pratique qui renvoie directement une `String` lisible, utile
/// pour afficher ou copier un champ texte extrait (les nombres et
/// booléens sont convertis en texte automatiquement).
pub fn resolve_as_string(value: &Value, path: &str) -> Option<String> {
    resolve(value, path).map(|v| match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    })
}

/// Variante qui renvoie un tableau de `Value`, utile pour `results_path`
/// qui doit pointer vers une liste de résultats de recherche.
pub fn resolve_as_array<'a>(value: &'a Value, path: &str) -> Option<&'a Vec<Value>> {
    resolve(value, path)?.as_array()
}