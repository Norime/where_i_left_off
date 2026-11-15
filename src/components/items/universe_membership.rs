use dioxus::prelude::*;

use crate::i18n::use_texts;
use crate::models::{entry_placement, Item, UniverseEntry};
use crate::routes::Route;
use crate::state::{ItemStore, UniverseStore};

/// Voisin d'une œuvre dans l'ordre affiché d'un univers.
#[derive(Clone, PartialEq)]
struct Neighbour {
    item_id: usize,
    category_id: usize,
    subcategory_id: usize,
    title: String,
}

fn neighbour(entry: Option<&UniverseEntry>, items: &[Item]) -> Option<Neighbour> {
    let entry = entry?;
    let (item, _) = entry_placement(entry, items)?;
    Some(Neighbour {
        item_id: item.id,
        category_id: entry.category_id,
        subcategory_id: entry.subcategory_id,
        title: item.title.clone(),
    })
}

/// Univers dont fait partie l'œuvre : position dans l'ordre affiché de
/// chacun, œuvres précédente et suivante, et ajout du placement affiché
/// (`category_id`/`subcategory_id`) à un autre univers. Rien n'est affiché
/// tant qu'aucun univers n'existe.
#[component]
pub fn UniverseMembership(item_id: usize, category_id: usize, subcategory_id: usize) -> Element {
    let i18n_text = use_texts();
    let mut universe_store = use_context::<Signal<UniverseStore>>();
    let item_store = use_context::<Signal<ItemStore>>();

    let items = item_store.read().items.clone();
    let universes = universe_store.read().universes.clone();
    if universes.is_empty() {
        return rsx! {};
    }

    // (univers, position, total, ordre, précédent, suivant)
    let memberships: Vec<(usize, String, usize, usize, String, Option<Neighbour>, Option<Neighbour>)> = universes
        .iter()
        .filter_map(|universe| {
            let order = universe.current_order()?;
            let ordered: Vec<UniverseEntry> =
                universe.ordered_entries(order, &items).into_iter().filter(|e| entry_placement(e, &items).is_some()).collect();
            let index = ordered.iter().position(|e| e.item_id == item_id)?;
            Some((
                universe.id,
                universe.name.clone(),
                index + 1,
                ordered.len(),
                order.name.clone(),
                neighbour(index.checked_sub(1).and_then(|i| ordered.get(i)), &items),
                neighbour(ordered.get(index + 1), &items),
            ))
        })
        .collect();
    let available: Vec<(usize, String)> = universes
        .iter()
        .filter(|u| !u.contains(item_id, category_id, subcategory_id))
        .map(|u| (u.id, u.name.clone()))
        .collect();

    rsx! {
        section { class: "universe-membership",
            h3 { "{i18n_text.universes}" }
            for (universe_id , name , position , total , order_name , previous , next) in memberships {
                div { key: "{universe_id}", class: "universe-membership-row",
                    Link { class: "universe-membership-name", to: Route::UniverseView { id: universe_id }, "{name}" }
                    span { class: "universe-row-meta", {(i18n_text.universe_position)(position, total, &order_name)} }
                    div { class: "universe-membership-nav",
                        if let Some(previous) = previous {
                            Link {
                                to: Route::ItemDetail {
                                    id: previous.item_id,
                                    category_id: previous.category_id,
                                    subcategory_id: previous.subcategory_id,
                                },
                                "‹ {previous.title}"
                            }
                        }
                        if let Some(next) = next {
                            Link {
                                class: "universe-membership-next",
                                to: Route::ItemDetail {
                                    id: next.item_id,
                                    category_id: next.category_id,
                                    subcategory_id: next.subcategory_id,
                                },
                                "{next.title} ›"
                            }
                        }
                    }
                }
            }
            if !available.is_empty() {
                select {
                    class: "universe-membership-add",
                    // Revient toujours sur l'invite : le choix ajoute tout de suite.
                    value: "",
                    onchange: move |event| {
                        if let Ok(universe_id) = event.value().parse::<usize>() {
                            universe_store.write().add_entry(universe_id, item_id, category_id, subcategory_id);
                        }
                    },
                    option { value: "", selected: true, "{i18n_text.add_to_universe}" }
                    for (universe_id , name) in available {
                        option { key: "{universe_id}", value: "{universe_id}", "{name}" }
                    }
                }
            }
        }
    }
}
