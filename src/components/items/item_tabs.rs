use dioxus::prelude::*;
use crate::components::{InlineAddForm, ProgressPanel};
use crate::models::ProgressNodeKind;
use crate::state::ItemStore;

/// Barre d'onglets horizontale au-dessus de la progression : "Progression"
/// est toujours présent en premier (pas stocké en base, pas supprimable),
/// suivi des onglets créés par l'utilisateur pour CE placement précis
/// (catégorie/sous-catégorie). Un onglet personnalisé n'a qu'un titre pour
/// l'instant : sa zone de contenu est vide, le contenu viendra plus tard.
#[component]
pub fn ItemTabs(item_id: usize, category_id: usize, subcategory_id: usize) -> Element {
    let mut item_store = use_context::<Signal<ItemStore>>();
    let mut active_tab = use_signal(|| None::<usize>);

    let tabs: Vec<(usize, String)> = item_store
        .read()
        .items
        .iter()
        .find(|i| i.id == item_id)
        .and_then(|item| {
            item.placements
                .iter()
                .find(|p| p.category_id == category_id && p.subcategory_id == subcategory_id)
        })
        .map(|p| p.tabs.iter().map(|t| (t.id, t.title.clone())).collect())
        .unwrap_or_default();

    // État de la progression affiché directement dans le nom de l'onglet
    // (ex: "Progression (5/12)"), pour ne plus avoir à le répéter dans la
    // zone en dessous.
    let (done_leaf_count, total_leaf_count): (usize, usize) = item_store
        .read()
        .items
        .iter()
        .find(|i| i.id == item_id)
        .and_then(|item| {
            item.placements
                .iter()
                .find(|p| p.category_id == category_id && p.subcategory_id == subcategory_id)
        })
        .map(|p| {
            let total = p.progress.iter().filter(|n| n.kind != ProgressNodeKind::Group).count();
            let done = p.progress.iter().filter(|n| n.kind != ProgressNodeKind::Group && n.is_done).count();
            (done, total)
        })
        .unwrap_or((0, 0));

    let progress_tab_label = if total_leaf_count > 0 {
        format!("Progression ({done_leaf_count}/{total_leaf_count})")
    } else {
        "Progression".to_string()
    };

    // Si l'onglet actif ne correspond plus à un onglet existant de CE
    // placement (par ex. après avoir changé de catégorie/sous-catégorie
    // dans le sélecteur au-dessus, ou après suppression de l'onglet), on
    // retombe sur "Progression" plutôt que de rester bloqué sur un id
    // périmé.
    let effective_active_tab = active_tab().filter(|active_id| tabs.iter().any(|(tab_id, _)| tab_id == active_id));

    rsx! {
        div { class: "item-tabs",
            div { class: "item-tabs-bar",
                button {
                    r#type: "button",
                    class: if effective_active_tab.is_none() { "item-tab item-tab--active" } else { "item-tab" },
                    onclick: move |_| active_tab.set(None),
                    "{progress_tab_label}"
                }
                for (tab_id , tab_title) in tabs {
                    div { class: "item-tab-wrapper", key: "{tab_id}",
                        button {
                            r#type: "button",
                            class: if effective_active_tab == Some(tab_id) { "item-tab item-tab--active" } else { "item-tab" },
                            onclick: move |_| active_tab.set(Some(tab_id)),
                            "{tab_title}"
                        }
                        button {
                            r#type: "button",
                            class: "item-tab-remove",
                            title: "Supprimer cet onglet",
                            onclick: move |_| {
                                item_store.write().remove_tab(item_id, category_id, subcategory_id, tab_id);
                                active_tab.set(None);
                            },
                            "×"
                        }
                    }
                }
                InlineAddForm {
                    placeholder: "Nouvel onglet",
                    onsubmit: move |title: String| {
                        let new_id = item_store
                            .write()
                            .add_tab(item_id, category_id, subcategory_id, title);
                        active_tab.set(new_id);
                    },
                }
            }
            div { class: "item-tabs-content",
                if let Some(tab_id) = effective_active_tab {
                    div { class: "item-tab-empty", key: "{tab_id}" }
                } else {
                    ProgressPanel { item_id, category_id, subcategory_id }
                }
            }
        }
    }
}