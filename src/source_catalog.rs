//! Catalogue de sources prêtes à l'emploi, proposées depuis l'onglet
//! « Sources » de l'accueil et depuis la gestion des sources d'une
//! sous-catégorie. Chaque entrée sait construire une `ApiSource` complète
//! (recherche + liste d'éléments) dans la langue voulue.
//!
//! Les gabarits utilisent des marqueurs remplacés UNE FOIS, à la création de
//! la source (`{lang}`, `{api_key}`...), à ne pas confondre avec les
//! marqueurs résolus à chaque requête (`{query}`, `{root.id}`, `{offset}`...).

use crate::i18n::Language;
use crate::models::{ApiSource, ListPagination, ProgressNodeKind, SourceField, SourceListLevel};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CatalogKind {
    Reading,
    Movies,
    Series,
}

impl CatalogKind {
    pub const ALL: [CatalogKind; 3] = [CatalogKind::Reading, CatalogKind::Movies, CatalogKind::Series];
}

pub struct CatalogSource {
    pub id: &'static str,
    pub kind: CatalogKind,
    /// Langues dans lesquelles la source fournit son contenu (titres,
    /// résumés, chapitres traduits...). La première sert par défaut.
    pub languages: &'static [Language],
    pub website: &'static str,
    /// Page où obtenir une clé gratuite, si la source en demande une.
    pub api_key_url: Option<&'static str>,
    description_fr: &'static str,
    description_en: &'static str,
    build: fn(Language) -> ApiSource,
}

impl CatalogSource {
    pub fn description(&self, ui_language: Language) -> &'static str {
        match ui_language {
            Language::Fr => self.description_fr,
            Language::En => self.description_en,
        }
    }

    pub fn supports(&self, language: Language) -> bool {
        self.languages.contains(&language)
    }

    /// Identifiant stocké dans `ApiSource::catalog_key` : une même source
    /// du catalogue dans deux langues donne deux sources distinctes.
    pub fn catalog_key(&self, content_language: Language) -> String {
        format!("{}:{}", self.id, content_language.code())
    }

    /// Nom affiché de la source créée (dans la langue de son contenu).
    pub fn name(&self, content_language: Language) -> String {
        (self.build)(content_language).name
    }

    /// Construit la source prête à être enregistrée (sans scope).
    pub fn build(&self, content_language: Language, api_key: &str) -> ApiSource {
        let mut source = (self.build)(content_language);
        let (tmdb, country, itunes) = match content_language {
            Language::Fr => ("fr-FR", "FR", "fr_fr"),
            Language::En => ("en-US", "US", "en_us"),
        };
        let vars = [
            ("{lang}", content_language.code()),
            ("{tmdb_lang}", tmdb),
            ("{country}", country),
            ("{itunes_lang}", itunes),
            ("{api_key}", api_key.trim()),
        ];
        let fill = |text: &mut String| {
            for (marker, value) in vars {
                *text = text.replace(marker, value);
            }
        };
        fill(&mut source.search_url_template);
        fill(&mut source.title_path);
        fill(&mut source.image_path);
        fill(&mut source.description_path);
        for level in source.list_levels.iter_mut() {
            fill(&mut level.url_template);
        }
        source.catalog_key = Some(self.catalog_key(content_language));
        source
    }
}

/// Toutes les sources du catalogue, dans l'ordre d'affichage.
pub fn catalog() -> &'static [CatalogSource] {
    &CATALOG
}

pub fn find(id: &str) -> Option<&'static CatalogSource> {
    CATALOG.iter().find(|entry| entry.id == id)
}

const FR_EN: &[Language] = &[Language::Fr, Language::En];
const EN: &[Language] = &[Language::En];
const FR: &[Language] = &[Language::Fr];

static CATALOG: [CatalogSource; 12] = [
    // --- Lecture ---
    CatalogSource {
        id: "mangadex",
        kind: CatalogKind::Reading,
        languages: FR_EN,
        website: "mangadex.org",
        api_key_url: None,
        description_fr: "Manga, manhwa et webtoons, avec la liste des chapitres traduits en français par la communauté.",
        description_en: "Manga, manhwa and webtoons, with the list of chapters translated by the community.",
        build: |lang| ApiSource {
            name: "MangaDex".into(),
            search_url_template: "https://api.mangadex.org/manga?title={query}&availableTranslatedLanguage[]={lang}&hasAvailableChapters=true&includes[]=cover_art&limit=20&order[followedCount]=desc".into(),
            results_path: "data".into(),
            title_path: "attributes.title.en|attributes.title.ja-ro|attributes.title.ko-ro|attributes.title.ja".into(),
            image_path: "https://uploads.mangadex.org/covers/{id}/{relationships[type=cover_art].attributes.fileName}.256.jpg".into(),
            description_path: "attributes.description.{lang}|attributes.description.en".into(),
            extra_fields: vec![
                field(t(lang, "Statut", "Status"), "attributes.status"),
                field(t(lang, "Année", "Year"), "attributes.year"),
                field(t(lang, "Dernier chapitre", "Last chapter"), "attributes.lastChapter"),
            ],
            list_levels: vec![SourceListLevel {
                label: t(lang, "Chapitres", "Chapters"),
                url_template: "https://api.mangadex.org/manga/{root.id}/feed?translatedLanguage[]={lang}&order[volume]=asc&order[chapter]=asc&limit=100&offset={offset}".into(),
                items_path: "data".into(),
                title_path: "Ch. {attributes.chapter}".into(),
                fields: vec![
                    field(t(lang, "Titre", "Title"), "attributes.title"),
                    field(t(lang, "Tome", "Volume"), "attributes.volume"),
                    field(t(lang, "Date de sortie", "Release date"), "attributes.publishAt"),
                ],
                leaf_kind: ProgressNodeKind::Text,
                pagination: ListPagination::Offset { page_size: 100 },
                ..Default::default()
            }],
            ..Default::default()
        },
    },
    CatalogSource {
        id: "apple_books",
        kind: CatalogKind::Reading,
        languages: FR_EN,
        website: "books.apple.com",
        api_key_url: None,
        description_fr: "Livres, romans, BD et mangas du catalogue Apple Books français (un résultat par tome), sans clé.",
        description_en: "Books, novels, comics and manga from the Apple Books catalog (one result per volume), no key needed.",
        build: |lang| ApiSource {
            name: "Apple Books".into(),
            search_url_template: "https://itunes.apple.com/search?term={query}&media=ebook&country={country}&lang={itunes_lang}&limit=25".into(),
            results_path: "results".into(),
            title_path: "trackName".into(),
            image_path: "artworkUrl100".into(),
            description_path: "description".into(),
            extra_fields: vec![
                field(t(lang, "Auteur", "Author"), "artistName"),
                field(t(lang, "Date de sortie", "Release date"), "releaseDate"),
                field(t(lang, "Genre", "Genre"), "genres[0]"),
            ],
            ..Default::default()
        },
    },
    CatalogSource {
        id: "google_books",
        kind: CatalogKind::Reading,
        languages: FR_EN,
        website: "books.google.com",
        api_key_url: Some("https://console.cloud.google.com/apis/library/books.googleapis.com"),
        description_fr: "Le catalogue Google Books, filtré sur les livres en français. Clé gratuite nécessaire (le quota sans clé est partagé et vite épuisé).",
        description_en: "The Google Books catalog, filtered on books in your language. Free key required (the keyless quota is shared and quickly exhausted).",
        build: |lang| ApiSource {
            name: "Google Books".into(),
            search_url_template: "https://www.googleapis.com/books/v1/volumes?q={query}&langRestrict={lang}&maxResults=20&key={api_key}".into(),
            results_path: "items".into(),
            title_path: "volumeInfo.title".into(),
            image_path: "volumeInfo.imageLinks.thumbnail".into(),
            description_path: "volumeInfo.description".into(),
            extra_fields: vec![
                field(t(lang, "Auteur", "Author"), "volumeInfo.authors[0]"),
                field(t(lang, "Date de sortie", "Release date"), "volumeInfo.publishedDate"),
                field("Pages".into(), "volumeInfo.pageCount"),
            ],
            ..Default::default()
        },
    },
    CatalogSource {
        id: "open_library",
        kind: CatalogKind::Reading,
        languages: EN,
        website: "openlibrary.org",
        api_key_url: None,
        description_fr: "Bibliothèque ouverte de l'Internet Archive : des millions de livres, surtout en anglais.",
        description_en: "Internet Archive's open library: millions of books.",
        build: |lang| ApiSource {
            name: "Open Library".into(),
            search_url_template: "https://openlibrary.org/search.json?q={query}&limit=20".into(),
            results_path: "docs".into(),
            title_path: "title".into(),
            image_path: "https://covers.openlibrary.org/b/id/{cover_i}-M.jpg".into(),
            extra_fields: vec![
                field(t(lang, "Auteur", "Author"), "author_name[0]"),
                field(t(lang, "Année", "Year"), "first_publish_year"),
            ],
            ..Default::default()
        },
    },
    CatalogSource {
        id: "bnf",
        kind: CatalogKind::Reading,
        languages: FR,
        website: "catalogue.bnf.fr",
        api_key_url: None,
        description_fr: "Catalogue général de la Bibliothèque nationale de France : livres, BD et manga publiés en France (dépôt légal), tome par tome. Pas de liste à suivre ; la couverture n'est pas toujours disponible.",
        description_en: "General catalogue of the French National Library: books, comics and manga published in France (legal deposit), volume by volume. No list to track; covers are not always available.",
        build: |lang| ApiSource {
            name: t(lang, "BnF – Catalogue général", "BnF – General catalogue"),
            // API SRU, réponse en XML (UNIMARC), convertie en JSON par
            // `xml_json`. `bib.doctype any "a"` : textes imprimés seulement.
            search_url_template: "https://catalogue.bnf.fr/api/SRU?version=1.2&operation=searchRetrieve&query=bib.title%20all%20%22{query}%22%20and%20bib.doctype%20any%20%22a%22&recordSchema=unimarcxchange&maximumRecords=20".into(),
            results_path: "records.record".into(),
            title_path: format!("{{{}}}. {{{}}}|{}", unimarc("200", "a"), unimarc("200", "h"), unimarc("200", "a")),
            image_path: "https://catalogue.bnf.fr/couverture?appName=NE&idArk={recordData.record.@id}&couverture=1".into(),
            description_path: unimarc("330", "a"),
            extra_fields: vec![
                field(
                    t(lang, "Auteur", "Author"),
                    &format!("{{{}}} {{{}}}|{}", unimarc("700", "b"), unimarc("700", "a"), unimarc("700", "a")),
                ),
                field(t(lang, "Éditeur", "Publisher"), &format!("{}|{}", unimarc("214", "c"), unimarc("210", "c"))),
                field(t(lang, "Date de publication", "Publication date"), &format!("{}|{}", unimarc("214", "d"), unimarc("210", "d"))),
                field(
                    t(lang, "Série", "Series"),
                    &format!("{{{}}}, {} {{{}}}|{}", unimarc("461", "t"), t(lang, "tome", "vol."), unimarc("461", "v"), unimarc("461", "t")),
                ),
                field("ISBN".into(), &unimarc("010", "a")),
            ],
            ..Default::default()
        },
    },
    CatalogSource {
        id: "kitsu_manga",
        kind: CatalogKind::Reading,
        languages: EN,
        website: "kitsu.app",
        api_key_url: None,
        description_fr: "Fiches manga de Kitsu (titre, résumé, nombre de tomes et de chapitres), en anglais.",
        description_en: "Manga entries from Kitsu (title, synopsis, volume and chapter counts).",
        build: |lang| ApiSource {
            name: "Kitsu Manga".into(),
            search_url_template: "https://kitsu.io/api/edge/manga?filter[text]={query}&page[limit]=20".into(),
            results_path: "data".into(),
            title_path: "attributes.titles.en|attributes.canonicalTitle".into(),
            image_path: "attributes.posterImage.small".into(),
            description_path: "attributes.synopsis".into(),
            extra_fields: vec![
                field(t(lang, "Tomes", "Volumes"), "attributes.volumeCount"),
                field(t(lang, "Chapitres", "Chapters"), "attributes.chapterCount"),
                field(t(lang, "Statut", "Status"), "attributes.status"),
            ],
            ..Default::default()
        },
    },
    // --- Films ---
    CatalogSource {
        id: "tmdb_movies",
        kind: CatalogKind::Movies,
        languages: FR_EN,
        website: "themoviedb.org",
        api_key_url: Some("https://www.themoviedb.org/settings/api"),
        description_fr: "La base de films la plus complète, avec titres, affiches et résumés en français. Clé gratuite nécessaire.",
        description_en: "The most complete movie database, with titles, posters and synopses. Free key required.",
        build: |lang| ApiSource {
            name: t(lang, "TMDB Films", "TMDB Movies"),
            search_url_template: "https://api.themoviedb.org/3/search/movie?api_key={api_key}&language={tmdb_lang}&query={query}".into(),
            results_path: "results".into(),
            title_path: "title".into(),
            image_path: "https://image.tmdb.org/t/p/w342{poster_path}".into(),
            description_path: "overview".into(),
            extra_fields: vec![
                field(t(lang, "Date de sortie", "Release date"), "release_date"),
                field(t(lang, "Titre original", "Original title"), "original_title"),
                field(t(lang, "Note", "Rating"), "vote_average"),
            ],
            ..Default::default()
        },
    },
    CatalogSource {
        id: "omdb_movies",
        kind: CatalogKind::Movies,
        languages: EN,
        website: "omdbapi.com",
        api_key_url: Some("https://www.omdbapi.com/apikey.aspx"),
        description_fr: "Open Movie Database (données IMDb), en anglais. Clé gratuite nécessaire.",
        description_en: "Open Movie Database (IMDb data). Free key required.",
        build: |lang| ApiSource {
            name: "OMDb".into(),
            search_url_template: "https://www.omdbapi.com/?apikey={api_key}&type=movie&s={query}".into(),
            results_path: "Search".into(),
            title_path: "Title".into(),
            image_path: "Poster".into(),
            extra_fields: vec![field(t(lang, "Année", "Year"), "Year"), field("IMDb".into(), "imdbID")],
            ..Default::default()
        },
    },
    // --- Séries ---
    CatalogSource {
        id: "tmdb_series",
        kind: CatalogKind::Series,
        languages: FR_EN,
        website: "themoviedb.org",
        api_key_url: Some("https://www.themoviedb.org/settings/api"),
        description_fr: "Séries et anime avec saisons et épisodes (titres, images, dates de diffusion) en français. Clé gratuite nécessaire.",
        description_en: "TV shows and anime with seasons and episodes (titles, stills, air dates). Free key required.",
        build: |lang| ApiSource {
            name: t(lang, "TMDB Séries", "TMDB TV shows"),
            search_url_template: "https://api.themoviedb.org/3/search/tv?api_key={api_key}&language={tmdb_lang}&query={query}".into(),
            results_path: "results".into(),
            title_path: "name".into(),
            image_path: "https://image.tmdb.org/t/p/w342{poster_path}".into(),
            description_path: "overview".into(),
            extra_fields: vec![
                field(t(lang, "Première diffusion", "First aired"), "first_air_date"),
                field(t(lang, "Titre original", "Original title"), "original_name"),
                field(t(lang, "Note", "Rating"), "vote_average"),
            ],
            list_levels: vec![
                SourceListLevel {
                    label: t(lang, "Saisons", "Seasons"),
                    url_template: "https://api.themoviedb.org/3/tv/{root.id}?api_key={api_key}&language={tmdb_lang}".into(),
                    items_path: "seasons".into(),
                    title_path: "name".into(),
                    image_path: "https://image.tmdb.org/t/p/w300{poster_path}".into(),
                    ..Default::default()
                },
                SourceListLevel {
                    label: t(lang, "Épisodes", "Episodes"),
                    url_template: "https://api.themoviedb.org/3/tv/{root.id}/season/{season_number}?api_key={api_key}&language={tmdb_lang}".into(),
                    items_path: "episodes".into(),
                    title_path: "{episode_number}. {name}".into(),
                    image_path: "https://image.tmdb.org/t/p/w300{still_path}".into(),
                    fields: vec![
                        field(t(lang, "Date de sortie", "Release date"), "air_date"),
                        field(t(lang, "Durée", "Duration"), "runtime"),
                    ],
                    leaf_kind: ProgressNodeKind::Card,
                    ..Default::default()
                },
            ],
            ..Default::default()
        },
    },
    CatalogSource {
        id: "tvmaze",
        kind: CatalogKind::Series,
        languages: EN,
        website: "tvmaze.com",
        api_key_url: None,
        description_fr: "Séries TV avec saisons, épisodes et dates de diffusion, sans clé. Contenu en anglais.",
        description_en: "TV shows with seasons, episodes and air dates, no key needed.",
        build: |lang| ApiSource {
            name: "TVmaze".into(),
            search_url_template: "https://api.tvmaze.com/search/shows?q={query}".into(),
            results_path: String::new(),
            title_path: "show.name".into(),
            image_path: "show.image.medium".into(),
            description_path: "show.summary".into(),
            extra_fields: vec![
                field(t(lang, "Statut", "Status"), "show.status"),
                field(t(lang, "Première diffusion", "Premiered"), "show.premiered"),
                field(t(lang, "Chaîne", "Network"), "show.network.name|show.webChannel.name"),
            ],
            list_levels: vec![
                SourceListLevel {
                    label: t(lang, "Saisons", "Seasons"),
                    url_template: "https://api.tvmaze.com/shows/{root.show.id}/seasons".into(),
                    title_path: t(lang, "Saison {number}", "Season {number}"),
                    image_path: "image.medium".into(),
                    ..Default::default()
                },
                SourceListLevel {
                    label: t(lang, "Épisodes", "Episodes"),
                    url_template: "https://api.tvmaze.com/seasons/{id}/episodes".into(),
                    title_path: "{number}. {name}".into(),
                    image_path: "image.medium".into(),
                    fields: vec![
                        field(t(lang, "Date de sortie", "Release date"), "airdate"),
                        field(t(lang, "Durée", "Duration"), "runtime"),
                    ],
                    leaf_kind: ProgressNodeKind::Card,
                    ..Default::default()
                },
            ],
            ..Default::default()
        },
    },
    CatalogSource {
        id: "kitsu_anime",
        kind: CatalogKind::Series,
        languages: EN,
        website: "kitsu.app",
        api_key_url: None,
        description_fr: "Anime avec la liste des épisodes (titres, miniatures, dates), sans clé. Contenu en anglais.",
        description_en: "Anime with their episode list (titles, thumbnails, dates), no key needed.",
        build: |lang| ApiSource {
            name: "Kitsu Anime".into(),
            search_url_template: "https://kitsu.io/api/edge/anime?filter[text]={query}&page[limit]=20".into(),
            results_path: "data".into(),
            title_path: "attributes.titles.en|attributes.canonicalTitle".into(),
            image_path: "attributes.posterImage.small".into(),
            description_path: "attributes.synopsis".into(),
            extra_fields: vec![
                field(t(lang, "Épisodes", "Episodes"), "attributes.episodeCount"),
                field(t(lang, "Statut", "Status"), "attributes.status"),
                field(t(lang, "Début", "Start date"), "attributes.startDate"),
            ],
            list_levels: vec![SourceListLevel {
                label: t(lang, "Épisodes", "Episodes"),
                url_template: "https://kitsu.io/api/edge/anime/{root.id}/episodes?page[limit]=20&page[offset]={offset}&sort=number".into(),
                items_path: "data".into(),
                title_path: "{attributes.number}. {attributes.canonicalTitle}".into(),
                image_path: "attributes.thumbnail.original".into(),
                fields: vec![
                    field(t(lang, "Date de sortie", "Release date"), "attributes.airdate"),
                    field(t(lang, "Durée", "Duration"), "attributes.length"),
                ],
                leaf_kind: ProgressNodeKind::Card,
                pagination: ListPagination::Offset { page_size: 20 },
                ..Default::default()
            }],
            ..Default::default()
        },
    },
    CatalogSource {
        id: "jikan_anime",
        kind: CatalogKind::Series,
        languages: EN,
        website: "myanimelist.net",
        api_key_url: None,
        description_fr: "Anime de MyAnimeList (via Jikan) avec la liste des épisodes, sans clé. Contenu en anglais ; l'API est parfois lente.",
        description_en: "Anime from MyAnimeList (via Jikan) with the episode list, no key needed. The API is sometimes slow.",
        build: |lang| ApiSource {
            name: "MyAnimeList (Jikan)".into(),
            search_url_template: "https://api.jikan.moe/v4/anime?q={query}&limit=20".into(),
            results_path: "data".into(),
            title_path: "title_english|title".into(),
            image_path: "images.jpg.image_url".into(),
            description_path: "synopsis".into(),
            extra_fields: vec![
                field(t(lang, "Épisodes", "Episodes"), "episodes"),
                field(t(lang, "Statut", "Status"), "status"),
            ],
            list_levels: vec![SourceListLevel {
                label: t(lang, "Épisodes", "Episodes"),
                url_template: "https://api.jikan.moe/v4/anime/{root.mal_id}/episodes?page={page}".into(),
                items_path: "data".into(),
                title_path: "{mal_id}. {title}".into(),
                fields: vec![field(t(lang, "Date de sortie", "Release date"), "aired")],
                leaf_kind: ProgressNodeKind::Text,
                pagination: ListPagination::Page { first: 1 },
                ..Default::default()
            }],
            ..Default::default()
        },
    },
];

fn t(lang: Language, fr: &str, en: &str) -> String {
    match lang {
        Language::Fr => fr.to_string(),
        Language::En => en.to_string(),
    }
}

/// Chemin vers une sous-zone d'une notice UNIMARC de la BnF, ex: zone 200
/// (titre), sous-zone `a` (titre propre).
fn unimarc(tag: &str, code: &str) -> String {
    format!("recordData.record.datafield[@tag={tag}].subfield[@code={code}]")
}

fn field(label: String, json_path: &str) -> SourceField {
    SourceField { label, json_path: json_path.to_string() }
}

