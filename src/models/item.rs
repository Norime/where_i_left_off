use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ItemOrientation {
    Vertical,
    Horizontal,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ItemStatus {
    New,
    InProgress,
    Done,
}

impl ItemStatus {
    pub fn value_key(&self) -> &'static str {
        match self {
            ItemStatus::New => "new",
            ItemStatus::InProgress => "in_progress",
            ItemStatus::Done => "done",
        }
    }

    pub fn rank(&self) -> u8 {
        match self {
            ItemStatus::New => 0,
            ItemStatus::InProgress => 1,
            ItemStatus::Done => 2,
        }
    }
}

/// Statut de l'ŒUVRE elle-même (sa diffusion/publication est-elle
/// terminée), à ne pas confondre avec `ItemStatus` qui suit plutôt LA
/// PROGRESSION de visionnage/lecture. Propre à chaque placement, comme
/// `type_ids`/`progress` : un même item peut être "non fini" côté anime
/// (abandonné) et "en cours" côté manga, ce sont deux œuvres distinctes qui
/// partagent juste la même fiche.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum WorkStatus {
    InProgress,
    Completed,
    Unfinished,
}

impl WorkStatus {
    pub fn value_key(&self) -> &'static str {
        match self {
            WorkStatus::InProgress => "in_progress",
            WorkStatus::Completed => "completed",
            WorkStatus::Unfinished => "unfinished",
        }
    }
}

impl Default for WorkStatus {
    fn default() -> Self {
        WorkStatus::InProgress
    }
}

/// Un onglet personnalisé ajouté par l'utilisateur à côté de l'onglet
/// "Progression" (toujours présent, pas stocké ici), propre à un placement
/// (catégorie/sous-catégorie). Pour l'instant un onglet n'a qu'un titre :
/// sa zone de contenu est vide, le contenu viendra plus tard.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ItemTab {
    pub id: usize,
    pub title: String,
}

/// Le type (ex: "Anime", "Roman") et la progression n'ont de sens que dans
/// une catégorie/sous-catégorie précise (un type appartient à une seule
/// catégorie ; une progression "épisodes" pour un anime n'a rien à voir
/// avec une progression "chapitres" pour un manga). Ils vivent donc ici,
/// sur le placement, et pas sur l'item lui-même.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ItemPlacement {
    pub category_id: usize,
    pub subcategory_id: usize,
    pub status: ItemStatus,
    #[serde(default)]
    pub type_ids: Vec<usize>,
    #[serde(default)]
    pub progress: Vec<ProgressNode>,
    #[serde(default)]
    pub work_status: WorkStatus,
    #[serde(default)]
    pub tabs: Vec<ItemTab>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ItemType {
    pub id: usize,
    pub category_id: usize,
    pub name: String,
}

/// Une information libre récupérée automatiquement (via une source
/// configurée) ou saisie à la main : un label ("Tomes", "Épisodes",
/// "Durée"...) et sa valeur ("12", "24", "42 min"...).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ItemInfoField {
    pub label: String,
    pub value: String,
}

/// Comment un nœud sans enfant doit s'afficher. Un nœud "Group" peut avoir
/// des enfants (une saison, un tome...) et ne compte jamais lui-même dans
/// le total de progression : seuls ses descendants "Text"/"Card" comptent.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ProgressNodeKind {
    /// Contient d'autres nœuds (saison, tome, coffret...). Pas de case à
    /// cocher qui compte dans le total, juste un conteneur dépliable.
    Group,
    /// Feuille affichée comme une simple ligne de texte (ex: un chapitre).
    Text,
    /// Feuille affichée comme une carte avec image, titre en haut et durée
    /// en bas à droite si un champ "Durée" existe (ex: un épisode).
    Card,
}

impl Default for ProgressNodeKind {
    fn default() -> Self {
        ProgressNodeKind::Text
    }
}

/// Un lien de redirection vers un site externe pour "regarder"/"lire" un
/// nœud de progression (ex: vers Crunchyroll, vers un site de scan...).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProgressLink {
    pub label: String,
    pub url: String,
}

/// Un nœud générique de suivi de progression : un film, une saison, un
/// épisode, un tome, un chapitre... N'importe quel "sous-élément" que
/// l'utilisateur veut cocher individuellement comme vu/lu.
///
/// La structure est à plat (comme les catégories et leurs sous-catégories)
/// plutôt que vraiment récursive en Rust : `parent_id` vaut `None` pour un
/// élément "racine", et `Some(id_du_parent)` pour un sous-élément d'un
/// autre nœud. Comme `parent_id` peut pointer vers N'IMPORTE QUEL nœud
/// (pas seulement une racine), la profondeur d'imbrication n'est pas
/// limitée : un nœud "Group" peut lui-même contenir un autre "Group".
///
/// `fields` reprend le même principe que `ItemInfoField` : des paires
/// label/valeur libres (date de sortie, durée, cast...), pour rester
/// adaptable à n'importe quel type de contenu sans rien coder en dur.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProgressNode {
    pub id: usize,
    pub parent_id: Option<usize>,
    #[serde(default)]
    pub kind: ProgressNodeKind,
    /// Seulement pertinent quand `kind == Group` : le type qu'auront TOUS
    /// les enfants de ce nœud, décidé une fois pour toutes à la création
    /// de la liste (impossible de mélanger texte et carte dans un même
    /// groupe). `None` pour un nœud créé avant l'ajout de ce champ, ou
    /// pour un nœud "Text"/"Card" (qui n'a pas d'enfants).
    #[serde(default)]
    pub children_kind: Option<ProgressNodeKind>,
    pub title: String,
    pub is_done: bool,
    #[serde(default)]
    pub image_url: Option<String>,
    pub fields: Vec<ItemInfoField>,
    /// Liens de redirection propres à CE nœud. Si vide, l'affichage
    /// remonte la chaîne des parents (`parent_id`) jusqu'à en trouver un
    /// non vide, pour permettre l'héritage ("hérite de son parent, et
    /// ainsi de suite").
    #[serde(default)]
    pub redirect_links: Vec<ProgressLink>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub id: usize,
    pub title: String,
    pub description: String,
    pub placements: Vec<ItemPlacement>,
    pub orientation: ItemOrientation,
    pub image_url: Option<String>,
    pub extra_info: Vec<ItemInfoField>,
}