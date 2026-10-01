use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde_json::Value;

use crate::models::ApiSource;

/// Une erreur de recherche, avec un message déjà prêt à afficher à
/// l'utilisateur (pas besoin de connaître les types internes de reqwest).
pub struct SearchError(pub String);

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

    let response = reqwest::get(&url)
        .await
        .map_err(|e| SearchError(format!("Impossible de contacter l'API : {e}")))?;

    if !response.status().is_success() {
        return Err(SearchError(format!(
            "L'API a répondu avec le code {}",
            response.status()
        )));
    }

    let body: Value = response
        .json()
        .await
        .map_err(|e| SearchError(format!("Réponse invalide (ce n'est pas du JSON) : {e}")))?;

    let results = crate::json_path::resolve_as_array(&body, &source.results_path)
        .cloned()
        .or_else(|| body.as_array().cloned())
        .unwrap_or_default();

    Ok(results)
}