use serde::{Deserialize, Serialize};

use super::ProgressNodeKind;

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
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
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
    /// Niveaux de la liste d'éléments à importer dans la progression
    /// (ex: [Saisons, Épisodes] ou juste [Chapitres]). Vide = pas d'import.
    #[serde(default)]
    pub list_levels: Vec<SourceListLevel>,
    /// Entrée du catalogue d'où vient la source, avec la langue de son
    /// contenu (ex: "tmdb_series:fr"). `None` pour une source faite main.
    #[serde(default)]
    pub catalog_key: Option<String>,
}

/// Comment parcourir les pages d'une liste quand l'API la découpe.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum ListPagination {
    /// Une seule requête.
    #[default]
    None,
    /// Le marqueur `{page}` vaut `first`, puis `first + 1`, etc.
    Page { first: usize },
    /// Le marqueur `{offset}` vaut 0, puis `page_size`, `2 * page_size`...
    Offset { page_size: usize },
}

/// Un niveau de la liste d'éléments importée depuis une source. Le niveau 1
/// part du résultat de recherche choisi, le niveau 2 part de chaque élément
/// du niveau 1, etc. Tous les niveaux sauf le dernier deviennent des
/// `ProgressNodeKind::Group`.
///
/// Les marqueurs `{chemin}` (dans l'URL, le titre ou l'image) sont résolus
/// sur l'élément parent ; `{root.chemin}` sur le résultat de recherche
/// d'origine ; `{page}` / `{offset}` servent à la pagination.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SourceListLevel {
    /// Nom du niveau ("Saisons", "Épisodes", "Chapitres"...).
    pub label: String,
    /// URL à appeler pour obtenir les éléments. Vide = les éléments sont
    /// déjà dans le JSON parent.
    pub url_template: String,
    /// Chemin vers le tableau d'éléments (vide = la réponse est le tableau).
    pub items_path: String,
    /// Chemin vers le titre, ou gabarit s'il contient `{` (ex:
    /// "Ch. {attributes.chapter} {attributes.title}").
    pub title_path: String,
    /// Chemin ou gabarit vers l'image (ex:
    /// "https://image.tmdb.org/t/p/w300{still_path}"). Vide = pas d'image.
    pub image_path: String,
    /// Champs libres à extraire sur chaque élément (durée, date...).
    pub fields: Vec<SourceField>,
    /// Affichage des éléments du DERNIER niveau (Text ou Card).
    pub leaf_kind: ProgressNodeKind,
    pub pagination: ListPagination,
}