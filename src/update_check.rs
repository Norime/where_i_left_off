use dioxus::prelude::*;

use crate::models::{ImportedFrom, WorkStatus};
use crate::net::SearchError;
use crate::source_import;
use crate::state::{ItemStore, SourceStore};

/// Demande envoyée au coroutine de vérification (voir `use_update_checker`).
pub struct StartUpdateCheck;

/// Avancement de la dernière vérification des nouveautés, partagé via le
/// contexte pour que l'accueil l'affiche même s'il a été quitté entre-temps.
#[derive(Clone, Debug, Default)]
pub struct UpdateCheckState {
    pub running: bool,
    pub done: usize,
    pub total: usize,
    pub current: Option<String>,
    /// Titre de l'œuvre + erreur rencontrée.
    pub errors: Vec<(String, SearchError)>,
}

/// Un placement à vérifier : œuvre encore "en cours" et reliée à une
/// source qui décrit une liste.
struct Candidate {
    item_id: usize,
    category_id: usize,
    subcategory_id: usize,
    title: String,
    imported_from: ImportedFrom,
}

/// Nombre de placements dont l'œuvre est encore en cours (ni terminée ni
/// non finie) : (reliés à une source utilisable, sans source).
pub fn count_ongoing(items: &ItemStore, sources: &SourceStore) -> (usize, usize) {
    let mut tracked = 0;
    let mut untracked = 0;
    for item in items.items.iter() {
        for placement in item.placements.iter().filter(|p| p.work_status == WorkStatus::InProgress) {
            if has_usable_source(placement.imported_from.as_ref(), sources) {
                tracked += 1;
            } else {
                untracked += 1;
            }
        }
    }
    (tracked, untracked)
}

fn has_usable_source(imported_from: Option<&ImportedFrom>, sources: &SourceStore) -> bool {
    imported_from.is_some_and(|from| {
        sources.sources.iter().any(|s| s.id == from.source_id && !s.list_levels.is_empty())
    })
}

/// Fournit `Signal<UpdateCheckState>` et un coroutine qui, à chaque
/// `StartUpdateCheck`, réimporte un par un tous les placements en cours
/// reliés à une source. Monté dans `AuthenticatedApp` : la vérification
/// continue quand on change de page.
pub fn use_update_checker(mut item_store: Signal<ItemStore>, source_store: Signal<SourceStore>) {
    let mut state = use_context_provider(|| Signal::new(UpdateCheckState::default()));

    use_coroutine(move |mut rx: UnboundedReceiver<StartUpdateCheck>| async move {
        while rx.recv().await.is_ok() {
            if state.peek().running {
                continue;
            }
            let candidates = collect_candidates(&item_store.peek(), &source_store.peek());
            state.set(UpdateCheckState { running: true, total: candidates.len(), ..Default::default() });

            for (index, candidate) in candidates.into_iter().enumerate() {
                state.write().current = Some(candidate.title.clone());
                let source = source_store
                    .peek()
                    .sources
                    .iter()
                    .find(|s| s.id == candidate.imported_from.source_id)
                    .cloned();
                if let Some(source) = source {
                    if index > 0 {
                        source_import::pause().await;
                    }
                    match source_import::fetch_progress(&source, &candidate.imported_from.root_result, source_import::MAX_PAGES).await {
                        Ok(fresh) => {
                            item_store.write().apply_reimport(
                                candidate.item_id,
                                candidate.category_id,
                                candidate.subcategory_id,
                                fresh,
                                true,
                            );
                        }
                        Err(e) => state.write().errors.push((candidate.title, e)),
                    }
                }
                state.write().done = index + 1;
            }

            let mut state = state.write();
            state.running = false;
            state.current = None;
        }
    });
}

fn collect_candidates(items: &ItemStore, sources: &SourceStore) -> Vec<Candidate> {
    items
        .items
        .iter()
        .flat_map(|item| {
            item.placements
                .iter()
                .filter(|p| p.work_status == WorkStatus::InProgress && has_usable_source(p.imported_from.as_ref(), sources))
                .filter_map(move |p| {
                    Some(Candidate {
                        item_id: item.id,
                        category_id: p.category_id,
                        subcategory_id: p.subcategory_id,
                        title: item.title.clone(),
                        imported_from: p.imported_from.clone()?,
                    })
                })
        })
        .collect()
}
