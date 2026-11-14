//! Conversion d'une réponse XML en `serde_json::Value`, pour que les
//! sources XML (ex: le catalogue SRU de la BnF) se lisent avec les mêmes
//! chemins que les sources JSON :
//! - un élément sans attribut ni enfant devient son texte ;
//! - sinon un objet : attributs en `"@nom"`, texte en `"#text"`, enfants par
//!   nom (sans préfixe d'espace de noms), en tableau s'ils se répètent ;
//! - la racine n'est pas enveloppée : `<a><b>x</b></a>` donne `{"b": "x"}`.

use serde_json::{Map, Value};

pub fn parse(text: &str) -> Result<Value, String> {
    let options = roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() };
    let document = roxmltree::Document::parse_with_options(text, options).map_err(|e| e.to_string())?;
    Ok(element_to_value(document.root_element()))
}

fn element_to_value(node: roxmltree::Node) -> Value {
    let mut map = Map::new();
    for attribute in node.attributes() {
        map.insert(format!("@{}", attribute.name()), Value::String(attribute.value().to_string()));
    }

    let mut text = String::new();
    for child in node.children() {
        if child.is_element() {
            let key = child.tag_name().name().to_string();
            let value = element_to_value(child);
            // Un élément n'est jamais converti en tableau : un tableau
            // existant veut donc dire que cet enfant se répète déjà.
            match map.get_mut(&key) {
                Some(Value::Array(list)) => list.push(value),
                Some(existing) => *existing = Value::Array(vec![existing.take(), value]),
                None => {
                    map.insert(key, value);
                }
            }
        } else if child.is_text() {
            text.push_str(child.text().unwrap_or_default());
        }
    }

    let text = text.trim();
    if map.is_empty() {
        return Value::String(text.to_string());
    }
    if !text.is_empty() {
        map.insert("#text".to_string(), Value::String(text.to_string()));
    }
    Value::Object(map)
}

#[cfg(test)]
mod tests {
    use super::parse;
    use serde_json::json;

    #[test]
    fn converts_attributes_text_and_repeated_children() {
        let xml = r#"<?xml version="1.0"?>
            <srw:response xmlns:srw="http://www.loc.gov/zing/srw/">
              <srw:count>2</srw:count>
              <field tag="200"><sub code="a">Frieren</sub><sub code="h">Vol. 6</sub></field>
              <field tag="214"><sub code="c">Ki-oon</sub></field>
            </srw:response>"#;
        assert_eq!(
            parse(xml).unwrap(),
            json!({
                "count": "2",
                "field": [
                    { "@tag": "200", "sub": [{ "@code": "a", "#text": "Frieren" }, { "@code": "h", "#text": "Vol. 6" }] },
                    { "@tag": "214", "sub": { "@code": "c", "#text": "Ki-oon" } }
                ]
            })
        );
    }
}
