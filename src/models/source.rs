use serde::{Deserialize, Serialize};

/// Un champ supplémentaire qu'une source sait extraire d'un résultat de
/// recherche : `label` est ce qui sera affiché ("Tomes", "Saisons"...),
/// `json_path` est le chemin vers la valeur dans la réponse JSON du
/// résultat choisi (ex: "volumes", "attributes.episodeCount").
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceField {
    pub label: String,
    pub json_path: String,
}

/// Une sous-catégorie précise (catégorie + sous-catégorie) à laquelle une
/// source est reliée. Une même source peut être reliée à plusieurs
/// `SourceScope`, dans la même catégorie ou dans des catégories
/// différentes, plutôt que d'être dupliquée à chaque fois.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceScope {
    pub category_id: usize,
    pub subcategory_id: usize,
}

/// Une source de données externe configurée par l'utilisateur pour une
/// SOUS-catégorie précise (pas toute la catégorie) : une API en lecture
/// seule (GET, sans authentification) qu'on interroge avec un titre, et
/// dont on sait extraire quelques champs à partir de chemins JSON définis
/// par l'utilisateur lui-même.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ApiSource {
    pub id: usize,
    pub linked_scopes: Vec<SourceScope>,
    pub name: String,
    /// URL de recherche contenant le marqueur `{query}`, remplacé par le
    /// titre recherché (encodé pour l'URL). Ex:
    /// "https://api.jikan.moe/v4/anime?q={query}"
    pub search_url_template: String,
    /// Chemin JSON vers le tableau de résultats dans la réponse
    /// (ex: "data"). Laisser vide si la réponse est déjà un tableau.
    pub results_path: String,
    /// Chemin JSON, à l'intérieur d'un résultat, vers son titre.
    pub title_path: String,
    /// Chemin JSON vers l'image de couverture. Vide = pas d'image.
    pub image_path: String,
    /// Chemin JSON vers un résumé/description. Vide = pas de description.
    pub description_path: String,
    /// Champs additionnels à extraire (tomes, épisodes, saisons...).
    pub extra_fields: Vec<SourceField>,
}