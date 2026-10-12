use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde_json::Value;

use crate::models::ApiSource;

/// Une erreur de recherche. Le message affiché est construit au moment de
/// l'affichage par `Texts::search_error`, donc dans la langue courante.
#[derive(Clone, Debug)]
pub enum SearchError {
    /// Impossible de joindre l'API (réseau, DNS, URL invalide...).
    Unreachable(String),
    /// L'API a répondu avec un code HTTP d'erreur.
    HttpStatus(String),
    /// La réponse n'est pas du JSON.
    InvalidJson(String),
}

/// Interroge la source configurée avec le titre donné et renvoie la liste
/// brute des résultats JSON trouvés (avant tout mapping de champs, qui se
/// fait ensuite via `json_path` sur le résultat choisi par l'utilisateur).
///
/// NOTE : ceci utilise `reqwest`, qui fait un vrai appel réseau HTTP.
/// Ça fonctionne tel quel sur desktop et mobile (Dioxus fournit déjà
/// l'exécuteur async nécessaire, `spawn` s'en sert). Sur la version web
/// (WASM), le navigateur bloquera probablement l'appel à cause de CORS si
/// l'API distante n'autorise pas explicitement les requêtes cross-origin :
/// ce sera à gérer plus tard avec un petit serveur relais si tu compiles
/// un jour pour le web.
pub async fn search(source: &ApiSource, query: &str) -> Result<Vec<Value>, SearchError> {
    let encoded_query = utf8_percent_encode(query, NON_ALPHANUMERIC).to_string();
    let url = source.search_url_template.replace("{query}", &encoded_query);

    let body = fetch_json(&url).await?;
    Ok(results_array(&body, &source.results_path))
}

/// Fait un GET sur `url` et renvoie le corps de la réponse en JSON.
pub async fn fetch_json(url: &str) -> Result<Value, SearchError> {
    let response = reqwest::get(url)
        .await
        .map_err(|e| SearchError::Unreachable(e.to_string()))?;

    if !response.status().is_success() {
        return Err(SearchError::HttpStatus(response.status().to_string()));
    }

    response
        .json()
        .await
        .map_err(|e| SearchError::InvalidJson(e.to_string()))
}

/// Le tableau trouvé à `path` dans `body`, ou `body` lui-même s'il est déjà
/// un tableau, sinon une liste vide.
pub fn results_array(body: &Value, path: &str) -> Vec<Value> {
    crate::json_path::resolve_as_array(body, path)
        .cloned()
        .or_else(|| body.as_array().cloned())
        .unwrap_or_default()
}

/// Remplace chaque marqueur `{...}` de `template` :
/// - `{nom}` présent dans `vars` (ex: `page`, `offset`) -> sa valeur ;
/// - `{root.chemin}` -> `chemin` résolu sur `root` ;
/// - `{chemin}` -> `chemin` résolu sur `parent`.
///
/// Un marqueur introuvable (ou `null`) est remplacé par du vide. Renvoie
/// aussi si TOUS les marqueurs ont été trouvés. `encode` encode les
/// valeurs JSON pour l'URL (pas celles de `vars`, qui sont des nombres).
pub fn fill_template(template: &str, parent: &Value, root: &Value, vars: &[(&str, String)], encode: bool) -> (String, bool) {
    let mut output = String::new();
    let mut all_found = true;
    let mut rest = template;

    while let Some(start) = rest.find('{') {
        let Some(len) = rest[start..].find('}') else {
            break;
        };
        output.push_str(&rest[..start]);
        let key = rest[start + 1..start + len].trim();

        let value = if let Some((_, value)) = vars.iter().find(|(name, _)| *name == key) {
            Some(value.clone())
        } else {
            let resolved = match key.strip_prefix("root.") {
                Some(path) => crate::json_path::resolve(root, path),
                None => crate::json_path::resolve(parent, key),
            };
            resolved.and_then(value_text).map(|text| {
                if encode {
                    utf8_percent_encode(&text, NON_ALPHANUMERIC).to_string()
                } else {
                    text
                }
            })
        };

        match value {
            Some(value) => output.push_str(&value),
            None => all_found = false,
        }
        rest = &rest[start + len + 1..];
    }

    output.push_str(rest);
    (output, all_found)
}

/// Texte lisible d'une valeur JSON ; `None` pour `null`.
pub fn value_text(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(s) => Some(s.clone()),
        other => Some(other.to_string()),
    }
}