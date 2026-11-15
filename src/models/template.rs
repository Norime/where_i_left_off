use crate::i18n::Texts;
use crate::models::{Item, ItemInfoField, ItemPlacement, ItemTab, ProgressMode, ProgressNode, ProgressNodeKind};

/// Modèle d'une sous-catégorie : prérègle les fiches qu'on y crée (mode de
/// suivi, première liste, infos globales et champs proposés, onglets,
/// vocabulaire, sources suggérées). Rien n'est figé : tout reste
/// modifiable sur la fiche, et changer de modèle ne touche que les
/// nouvelles fiches. Pour en ajouter un : un variant, une ligne dans `ALL`,
/// `key`, son contenu ci-dessous et son nom (`Texts::template_name`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubcategoryTemplate {
    Movie,
    Series,
    Manga,
    Novel,
    VideoGame,
    Podcast,
    Audiobook,
}

/// Info globale ou champ d'un élément proposé par un modèle. Le libellé
/// écrit dans la fiche est celui de la langue active à sa création
/// (`Texts::template_field`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TemplateField {
    Director,
    ReleaseDate,
    Duration,
    Studio,
    Author,
    Publisher,
    Pages,
    Platform,
    Host,
    Narrator,
}

/// Onglet créé d'office par un modèle (`Texts::template_tab`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TemplateTab {
    Cast,
    Characters,
    Achievements,
}

/// Première liste créée d'office (`Texts::template_group`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TemplateGroup {
    Season1,
    Volume1,
    Chapters,
    Episodes,
}

/// Mot employé pour « terminé » dans la sous-catégorie (`Texts::done_word`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DoneWord {
    #[default]
    Done,
    Watched,
    Read,
    Listened,
}

impl SubcategoryTemplate {
    /// Ordre d'affichage dans le choix du modèle.
    pub const ALL: [SubcategoryTemplate; 7] = [
        SubcategoryTemplate::Movie,
        SubcategoryTemplate::Series,
        SubcategoryTemplate::Manga,
        SubcategoryTemplate::Novel,
        SubcategoryTemplate::VideoGame,
        SubcategoryTemplate::Podcast,
        SubcategoryTemplate::Audiobook,
    ];

    /// Clé enregistrée dans la sous-catégorie.
    pub fn key(self) -> &'static str {
        match self {
            SubcategoryTemplate::Movie => "movie",
            SubcategoryTemplate::Series => "series",
            SubcategoryTemplate::Manga => "manga",
            SubcategoryTemplate::Novel => "novel",
            SubcategoryTemplate::VideoGame => "video_game",
            SubcategoryTemplate::Podcast => "podcast",
            SubcategoryTemplate::Audiobook => "audiobook",
        }
    }

    /// `None` pour une clé inconnue (modèle retiré de l'application).
    pub fn from_key(key: &str) -> Option<SubcategoryTemplate> {
        SubcategoryTemplate::ALL.into_iter().find(|t| t.key() == key)
    }

    pub fn progress_mode(self) -> ProgressMode {
        match self {
            SubcategoryTemplate::Movie | SubcategoryTemplate::Novel => ProgressMode::Single,
            SubcategoryTemplate::VideoGame => ProgressMode::Percent,
            _ => ProgressMode::List,
        }
    }

    /// Première liste et type de ses éléments.
    pub fn first_group(self) -> Option<(TemplateGroup, ProgressNodeKind)> {
        match self {
            SubcategoryTemplate::Series => Some((TemplateGroup::Season1, ProgressNodeKind::Card)),
            SubcategoryTemplate::Manga => Some((TemplateGroup::Volume1, ProgressNodeKind::Text)),
            SubcategoryTemplate::VideoGame | SubcategoryTemplate::Audiobook => Some((TemplateGroup::Chapters, ProgressNodeKind::Text)),
            SubcategoryTemplate::Podcast => Some((TemplateGroup::Episodes, ProgressNodeKind::Card)),
            SubcategoryTemplate::Movie | SubcategoryTemplate::Novel => None,
        }
    }

    /// Infos globales de l'œuvre proposées (vides) à la création.
    pub fn global_fields(self) -> &'static [TemplateField] {
        use TemplateField::*;
        match self {
            SubcategoryTemplate::Movie => &[Director, ReleaseDate, Duration],
            SubcategoryTemplate::Series => &[Studio, ReleaseDate],
            SubcategoryTemplate::Manga => &[Author, Publisher, ReleaseDate],
            SubcategoryTemplate::Novel => &[Author, Pages, ReleaseDate],
            SubcategoryTemplate::VideoGame => &[Studio, Platform, ReleaseDate],
            SubcategoryTemplate::Podcast => &[Host],
            SubcategoryTemplate::Audiobook => &[Author, Narrator, Duration],
        }
    }

    /// Champs proposés (vides) sur chaque élément ajouté à une liste.
    pub fn leaf_fields(self) -> &'static [TemplateField] {
        use TemplateField::*;
        match self {
            SubcategoryTemplate::Series => &[Duration, ReleaseDate],
            SubcategoryTemplate::Manga => &[ReleaseDate],
            SubcategoryTemplate::Podcast => &[Duration, ReleaseDate],
            SubcategoryTemplate::Audiobook => &[Duration],
            _ => &[],
        }
    }

    pub fn tabs(self) -> &'static [TemplateTab] {
        match self {
            SubcategoryTemplate::Movie => &[TemplateTab::Cast],
            SubcategoryTemplate::Series => &[TemplateTab::Cast, TemplateTab::Characters],
            SubcategoryTemplate::Manga => &[TemplateTab::Characters],
            SubcategoryTemplate::VideoGame => &[TemplateTab::Achievements],
            _ => &[],
        }
    }

    pub fn done_word(self) -> DoneWord {
        match self {
            SubcategoryTemplate::Movie | SubcategoryTemplate::Series => DoneWord::Watched,
            SubcategoryTemplate::Manga | SubcategoryTemplate::Novel => DoneWord::Read,
            SubcategoryTemplate::Podcast | SubcategoryTemplate::Audiobook => DoneWord::Listened,
            SubcategoryTemplate::VideoGame => DoneWord::Done,
        }
    }

    /// Sources du catalogue (`source_catalog`) adaptées au modèle.
    pub fn suggested_sources(self) -> &'static [&'static str] {
        match self {
            SubcategoryTemplate::Movie => &["tmdb_movies", "omdb_movies"],
            SubcategoryTemplate::Series => &["tmdb_series", "tvmaze", "kitsu_anime", "jikan_anime"],
            SubcategoryTemplate::Manga => &["mangadex", "kitsu_manga"],
            SubcategoryTemplate::Novel => &["open_library", "google_books", "apple_books", "bnf"],
            SubcategoryTemplate::VideoGame | SubcategoryTemplate::Podcast | SubcategoryTemplate::Audiobook => &[],
        }
    }

    /// Prérègle un nouveau placement et complète les infos de l'œuvre.
    /// Ne remplace rien : une liste importée depuis une source est gardée
    /// (et la fiche simple n'est alors pas appliquée, pour ne pas la
    /// masquer), les onglets existants aussi, et seules les infos
    /// absentes sont ajoutées, vides.
    pub fn apply(self, texts: &Texts, extra_info: &mut Vec<ItemInfoField>, placement: &mut ItemPlacement) {
        let mode = self.progress_mode();
        if mode != ProgressMode::Single || placement.progress.is_empty() {
            placement.progress_mode = mode;
        }
        if placement.progress.is_empty() {
            if let Some((group, children_kind)) = self.first_group() {
                placement.progress.push(ProgressNode {
                    id: 0,
                    parent_id: None,
                    kind: ProgressNodeKind::Group,
                    children_kind: Some(children_kind),
                    title: texts.template_group(group).to_string(),
                    is_done: false,
                    image_url: None,
                    fields: Vec::new(),
                    redirect_links: Vec::new(),
                    is_new: false,
                    new_since: None,
                    resume_at: None,
                });
            }
        }
        if placement.tabs.is_empty() {
            placement.tabs = self
                .tabs()
                .iter()
                .enumerate()
                .map(|(id, tab)| ItemTab { id, title: texts.template_tab(*tab).to_string() })
                .collect();
        }
        add_missing_fields(extra_info, self.global_fields().iter().map(|f| texts.template_field(*f).to_string()));
        placement.refresh_status();
    }
}

/// Ajoute, vides, les champs dont le libellé n'existe pas encore (sans
/// tenir compte des majuscules).
pub fn add_missing_fields(fields: &mut Vec<ItemInfoField>, labels: impl IntoIterator<Item = String>) {
    for label in labels {
        if !fields.iter().any(|f| f.label.trim().eq_ignore_ascii_case(label.trim())) {
            fields.push(ItemInfoField { label, value: String::new() });
        }
    }
}

/// Infos de l'œuvre complétées par le modèle (voir
/// `SubcategoryTemplate::apply`), pour un placement déjà dans `item`.
pub fn apply_to_item(template: SubcategoryTemplate, texts: &Texts, item: &mut Item, category_id: usize, subcategory_id: usize) {
    let Some(placement) = item.placements.iter_mut().find(|p| p.category_id == category_id && p.subcategory_id == subcategory_id) else {
        return;
    };
    template.apply(texts, &mut item.extra_info, placement);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Language;

    #[test]
    fn template_fills_without_overwriting() {
        let texts = Language::Fr.texts();
        let mut extra_info = vec![ItemInfoField { label: "studio".to_string(), value: "Bones".to_string() }];
        let mut placement = ItemPlacement::new(1, 2);
        SubcategoryTemplate::Series.apply(texts, &mut extra_info, &mut placement);
        assert_eq!(extra_info.len(), 2);
        assert_eq!(extra_info[0].value, "Bones");
        assert_eq!(extra_info[1].label, texts.template_field(TemplateField::ReleaseDate));
        assert_eq!(extra_info[1].value, "");
        assert_eq!(placement.progress.len(), 1);
        assert_eq!(placement.progress[0].children_kind, Some(ProgressNodeKind::Card));
        assert_eq!(placement.tabs.len(), 2);
    }

    #[test]
    fn imported_list_is_kept_and_not_hidden() {
        let texts = Language::Fr.texts();
        let mut extra_info = Vec::new();
        let mut placement = ItemPlacement::new(1, 2);
        let mut imported = ItemPlacement::new(1, 2);
        SubcategoryTemplate::Podcast.apply(texts, &mut extra_info, &mut imported);
        placement.progress = imported.progress.clone();
        placement.progress[0].title = "Saison importée".to_string();
        SubcategoryTemplate::Movie.apply(texts, &mut extra_info, &mut placement);
        assert_eq!(placement.progress_mode, ProgressMode::List);
        assert_eq!(placement.progress[0].title, "Saison importée");
    }

    #[test]
    fn unknown_key_means_no_template() {
        assert_eq!(SubcategoryTemplate::from_key("vinyl"), None);
        for template in SubcategoryTemplate::ALL {
            assert_eq!(SubcategoryTemplate::from_key(template.key()), Some(template));
        }
    }
}
