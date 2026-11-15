use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ItemOrientation {
    Vertical,
    Horizontal,
}

/// Où en est l'utilisateur dans un placement. Nouveau / En cours / Terminé
/// se calculent d'après la progression quand il y a une liste à cocher ;
/// À faire / En pause / Abandonné sont choisis à la main et gardés tels
/// quels (sauf À faire, qui passe en cours dès qu'un élément est coché).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ItemStatus {
    New,
    InProgress,
    Done,
    Planned,
    OnHold,
    Dropped,
}

impl ItemStatus {
    /// Ordre d'affichage dans les listes de choix.
    pub const ALL: [ItemStatus; 6] = [
        ItemStatus::New,
        ItemStatus::Planned,
        ItemStatus::InProgress,
        ItemStatus::OnHold,
        ItemStatus::Done,
        ItemStatus::Dropped,
    ];

    /// Statuts qu'on ne peut que choisir à la main.
    pub const MANUAL: [ItemStatus; 3] = [ItemStatus::Planned, ItemStatus::OnHold, ItemStatus::Dropped];

    pub fn value_key(&self) -> &'static str {
        match self {
            ItemStatus::New => "new",
            ItemStatus::InProgress => "in_progress",
            ItemStatus::Done => "done",
            ItemStatus::Planned => "planned",
            ItemStatus::OnHold => "on_hold",
            ItemStatus::Dropped => "dropped",
        }
    }

    pub fn from_key(key: &str) -> Option<ItemStatus> {
        ItemStatus::ALL.into_iter().find(|s| s.value_key() == key)
    }

    pub fn is_manual(&self) -> bool {
        ItemStatus::MANUAL.contains(self)
    }

    pub fn rank(&self) -> u8 {
        match self {
            ItemStatus::New => 0,
            ItemStatus::InProgress => 1,
            ItemStatus::Done => 2,
            ItemStatus::Planned => 0,
            ItemStatus::OnHold => 1,
            ItemStatus::Dropped => 1,
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

/// Comment se suit un placement : une liste d'éléments à cocher (épisodes,
/// chapitres…), ou une fiche simple sans liste, juste vue ou pas (un film).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum ProgressMode {
    #[default]
    List,
    Single,
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
    /// Source qui a rempli la progression, pour pouvoir la réimporter.
    #[serde(default)]
    pub imported_from: Option<ImportedFrom>,
    #[serde(default)]
    pub progress_mode: ProgressMode,
}

impl ItemPlacement {
    /// Placement vide (statut Nouveau, pas de type ni de progression).
    pub fn new(category_id: usize, subcategory_id: usize) -> Self {
        ItemPlacement {
            category_id,
            subcategory_id,
            status: ItemStatus::New,
            type_ids: Vec::new(),
            progress: Vec::new(),
            work_status: WorkStatus::default(),
            tabs: Vec::new(),
            imported_from: None,
            progress_mode: ProgressMode::default(),
        }
    }

    /// Éléments cochables (Text/Card) de la progression.
    pub fn leaves(&self) -> impl Iterator<Item = &ProgressNode> {
        self.progress.iter().filter(|n| n.kind != ProgressNodeKind::Group)
    }

    /// Statut déduit de la progression : `None` en fiche simple ou sans
    /// élément à cocher, où le statut ne se choisit qu'à la main.
    pub fn computed_status(&self) -> Option<ItemStatus> {
        if self.progress_mode != ProgressMode::List {
            return None;
        }
        let total = self.leaves().count();
        let done = self.leaves().filter(|n| n.is_done).count();
        match (total, done) {
            (0, _) => None,
            (_, 0) => Some(ItemStatus::New),
            (total, done) if done < total => Some(ItemStatus::InProgress),
            _ => Some(ItemStatus::Done),
        }
    }

    /// Recalcule le statut d'après la progression. En pause et Abandonné
    /// restent tels quels ; À faire le reste tant que rien n'est coché.
    pub fn refresh_status(&mut self) {
        let Some(computed) = self.computed_status() else { return };
        self.status = match self.status {
            ItemStatus::OnHold | ItemStatus::Dropped => self.status,
            ItemStatus::Planned if computed == ItemStatus::New => ItemStatus::Planned,
            _ => computed,
        };
    }
}

/// D'où vient la progression importée d'un placement : la source utilisée
/// et le résultat de recherche choisi, gardé tel quel pour pouvoir
/// réimporter plus tard (nouveaux épisodes...) sans refaire la recherche.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImportedFrom {
    pub source_id: usize,
    pub root_result: serde_json::Value,
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
    /// Apparu lors d'une réimportation (nouvel épisode, chapitre...) et pas
    /// encore vu : remis à `false` dès qu'on le coche ou qu'on marque
    /// l'œuvre comme vue depuis l'accueil.
    #[serde(default)]
    pub is_new: bool,
    /// Quand le nœud a été détecté comme nouveau (secondes Unix) : sert à ne
    /// garder dans les nouveautés de l'accueil que les détections récentes.
    #[serde(default)]
    pub new_since: Option<u64>,
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
#[cfg(test)]
mod tests {
    use super::*;

    fn leaf(id: usize, is_done: bool) -> ProgressNode {
        ProgressNode {
            id,
            parent_id: None,
            kind: ProgressNodeKind::Text,
            children_kind: None,
            title: format!("{id}"),
            is_done,
            image_url: None,
            fields: Vec::new(),
            redirect_links: Vec::new(),
            is_new: false,
            new_since: None,
        }
    }

    #[test]
    fn manual_statuses_survive_progress_changes() {
        let mut placement = ItemPlacement::new(1, 1);
        placement.progress = vec![leaf(0, false), leaf(1, false)];

        placement.status = ItemStatus::Planned;
        placement.refresh_status();
        assert_eq!(placement.status, ItemStatus::Planned);
        placement.progress[0].is_done = true;
        placement.refresh_status();
        assert_eq!(placement.status, ItemStatus::InProgress);

        placement.status = ItemStatus::OnHold;
        placement.progress[1].is_done = true;
        placement.refresh_status();
        assert_eq!(placement.status, ItemStatus::OnHold);

        placement.status = ItemStatus::Dropped;
        placement.refresh_status();
        assert_eq!(placement.status, ItemStatus::Dropped);
    }

    #[test]
    fn single_mode_status_is_never_computed() {
        let mut placement = ItemPlacement::new(1, 1);
        placement.progress_mode = ProgressMode::Single;
        placement.status = ItemStatus::Done;
        placement.refresh_status();
        assert_eq!(placement.status, ItemStatus::Done);
        assert_eq!(placement.computed_status(), None);
    }

    #[test]
    fn old_placements_load_in_list_mode() {
        let placement: ItemPlacement =
            serde_json::from_str(r#"{ "category_id": 1, "subcategory_id": 2, "status": "InProgress" }"#).unwrap();
        assert_eq!(placement.progress_mode, ProgressMode::List);
        assert_eq!(placement.status, ItemStatus::InProgress);
    }
}
