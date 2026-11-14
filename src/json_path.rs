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
///   "relationships[type=cover_art].id"
///                            -> "id" du premier élément de "relationships"
///                               dont le champ "type" vaut "cover_art"
///   "datafield[@tag=200]"    -> pareil sur un attribut XML (voir `xml_json`)
///   "title.fr|title.en"      -> le premier chemin qui donne une valeur
///                               non nulle (alternatives séparées par `|`)
///
/// Un chemin vide ("") renvoie la valeur elle-même (utile quand la donnée
/// est directement à la racine, par exemple pour `results_path` quand la
/// réponse EST déjà le tableau de résultats).
pub fn resolve<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    if path.contains('|') {
        return path.split('|').find_map(|alternative| resolve(value, alternative).filter(|v| !v.is_null()));
    }

    let path = path.trim();
    if path.is_empty() {
        return Some(value);
    }

    let mut current = value;
    for raw_segment in path.split('.') {
        let (field_part, selectors) = split_indices(raw_segment);

        if !field_part.is_empty() {
            current = current.as_object()?.get(field_part)?;
        }

        for selector in selectors {
            // Une valeur seule compte comme une liste d'un élément (en XML,
            // un élément qui ne se répète pas n'est pas un tableau).
            let array = match current {
                Value::Array(array) => array.as_slice(),
                single => std::slice::from_ref(single),
            };
            current = match selector {
                Selector::Index(idx) => array.get(idx)?,
                Selector::Filter(key, expected) => array.iter().find(|element| {
                    element.get(key).and_then(crate::net::value_text).is_some_and(|text| text == expected)
                })?,
            };
        }
    }

    Some(current)
}

/// Ce qu'il y a entre crochets : un indice (`[0]`) ou un filtre
/// (`[type=cover_art]`, premier élément dont le champ vaut ce texte).
enum Selector<'a> {
    Index(usize),
    Filter(&'a str, &'a str),
}

/// Sépare "genres[0][type=x]" en ("genres", [Index(0), Filter("type", "x")]).
/// "[0]" seul donne ("", [Index(0)]).
fn split_indices(segment: &str) -> (&str, Vec<Selector<'_>>) {
    let bracket_pos = segment.find('[');
    let (field, mut rest) = match bracket_pos {
        Some(pos) => (&segment[..pos], &segment[pos..]),
        None => (segment, ""),
    };

    let mut indices = Vec::new();
    while let Some(start) = rest.find('[') {
        if let Some(end) = rest[start..].find(']') {
            let end = start + end;
            let inside = rest[start + 1..end].trim();
            if let Ok(idx) = inside.parse::<usize>() {
                indices.push(Selector::Index(idx));
            } else if let Some((key, expected)) = inside.split_once('=') {
                indices.push(Selector::Filter(key.trim(), expected.trim()));
            }
            rest = &rest[end + 1..];
        } else {
            break;
        }
    }

    (field, indices)
}

/// Variante qui renvoie un tableau de `Value`, utile pour `results_path`
/// qui doit pointer vers une liste de résultats de recherche.
pub fn resolve_as_array<'a>(value: &'a Value, path: &str) -> Option<&'a Vec<Value>> {
    resolve(value, path)?.as_array()
}
#[cfg(test)]
mod tests {
    use super::resolve;
    use serde_json::json;

    #[test]
    fn filter_selects_first_matching_element() {
        let value = json!({ "relationships": [
            { "type": "author", "id": "a" },
            { "type": "cover_art", "attributes": { "fileName": "cover.jpg" } }
        ] });
        assert_eq!(resolve(&value, "relationships[type=cover_art].attributes.fileName"), Some(&json!("cover.jpg")));
        assert_eq!(resolve(&value, "relationships[type=missing].id"), None);
        assert_eq!(resolve(&value, "relationships[0].id"), Some(&json!("a")));
    }

    #[test]
    fn selectors_treat_single_value_as_one_element_list() {
        let value = json!({ "sub": { "@code": "a", "#text": "Frieren" } });
        assert_eq!(resolve(&value, "sub[@code=a].#text"), Some(&json!("Frieren")));
        assert_eq!(resolve(&value, "sub[0].@code"), Some(&json!("a")));
        assert_eq!(resolve(&value, "sub[@code=h]"), None);
    }

    #[test]
    fn alternatives_take_first_non_null_value() {
        let value = json!({ "title": { "ja-ro": "Sousou no Frieren" }, "english": null });
        assert_eq!(resolve(&value, "title.en|title.ja-ro"), Some(&json!("Sousou no Frieren")));
        assert_eq!(resolve(&value, "english|title.ja-ro"), Some(&json!("Sousou no Frieren")));
        assert_eq!(resolve(&value, "title.en|title.fr"), None);
    }
}
