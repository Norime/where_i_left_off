use dioxus::prelude::*;
use std::collections::HashSet;

use crate::components::{ItemCard, ManageSourcesModal, NewItemModal, TypeFilterDropdown};
use crate::models::ItemStatus;
use crate::state::ItemStore;

const BATCH_SIZE: usize = 24;

#[derive(Clone, Copy, PartialEq)]
enum SortOrder {
    TitleAsc,
    TitleDesc,
}

#[component]
pub fn CategoryView(category_id: usize, subcategory_id: usize) -> Element {
    let item_store = use_context::<Signal<ItemStore>>();

    let mut sort_order = use_signal(|| SortOrder::TitleAsc);
    let mut status_filter = use_signal(|| Option::<ItemStatus>::None);
    let type_filter = use_signal(HashSet::<usize>::new);
    let mut visible_count = use_signal(|| BATCH_SIZE);
    let mut show_new_item_modal = use_signal(|| false);
    let mut show_sources_modal = use_signal(|| false);

    let mut search_query = use_signal(String::new);

    use_effect(move || {
        let _ = category_id;
        let _ = subcategory_id;
        let _ = *status_filter.read();
        let _ = type_filter.read().clone();
        let _ = *sort_order.read();
        visible_count.set(BATCH_SIZE);
    });

    use_effect(move || {
        let _ = category_id;
        let _ = subcategory_id;
        let _ = *status_filter.read();
        let _ = type_filter.read().clone();
        let _ = *sort_order.read();
        let _ = search_query.read().clone();
        visible_count.set(BATCH_SIZE);
    });

    let mut matching_items: Vec<_> = item_store
        .read()
        .items
        .iter()
        .filter(|item| {
            item.placements.iter().any(|p| {
                p.category_id == category_id
                    && (subcategory_id == 0 || p.subcategory_id == subcategory_id)
            })
        })
        .filter(|item| match *status_filter.read() {
            None => true,
            Some(wanted) => item.placements.iter().any(|p| {
                p.category_id == category_id
                    && (subcategory_id == 0 || p.subcategory_id == subcategory_id)
                    && p.status == wanted
            }),
        })
        .filter(|item| {
            let wanted_types = type_filter.read();
            wanted_types.is_empty()
                || item.placements.iter().any(|p| {
                    p.category_id == category_id
                        && (subcategory_id == 0 || p.subcategory_id == subcategory_id)
                        && p.type_ids.iter().any(|t| wanted_types.contains(t))
                })
        })
        .filter(|item| {
            let query = search_query.read();
            let needle = query.trim().to_lowercase();
            needle.is_empty() || item.title.to_lowercase().contains(&needle)
        })
        .cloned()
        .collect();

    match *sort_order.read() {
        SortOrder::TitleAsc => matching_items.sort_by(|a, b| a.title.cmp(&b.title)),
        SortOrder::TitleDesc => matching_items.sort_by(|a, b| b.title.cmp(&a.title)),
    }

    let total_count = matching_items.len();
    let shown_count = (*visible_count.read()).min(total_count);
    let visible_items = matching_items[..shown_count].to_vec();
    let has_more = shown_count < total_count;
    let is_specific_subcategory = subcategory_id != 0;

    rsx! {
        div { class: "category-view",
            div { class: "category-view-toolbar",
                // Trie par ordre alphabétique
                select {
                    class: "toolbar-control",
                    onchange: move |event| {
                        sort_order
                            .set(
                                if event.value() == "desc" {
                                    SortOrder::TitleDesc
                                } else {
                                    SortOrder::TitleAsc
                                },
                            );
                    },
                    option { value: "asc", "Titre A → Z" }
                    option { value: "desc", "Titre Z → A" }
                }
                // Trie par status
                select {
                    class: "toolbar-control",
                    onchange: move |event| {
                        status_filter
                            .set(
                                match event.value().as_str() {
                                    "new" => Some(ItemStatus::New),
                                    "in_progress" => Some(ItemStatus::InProgress),
                                    "done" => Some(ItemStatus::Done),
                                    _ => None,
                                },
                            );
                    },
                    option { value: "all", "Statut : Tous" }
                    option { value: "new", "Nouveau" }
                    option { value: "in_progress", "En cours" }
                    option { value: "done", "Terminé" }
                }
                // Combobox de filtre des Types
                TypeFilterDropdown { category_ids: vec![category_id], selected: type_filter }
                // Barre de recherche
                input {
                    r#type: "text",
                    class: "toolbar-control search-input",
                    placeholder: "Rechercher un titre...",
                    value: "{search_query()}",
                    oninput: move |event| search_query.set(event.value()),
                }
                // Source API de récupération d'info
                if is_specific_subcategory {
                    button {
                        r#type: "button",
                        class: "secondary-btn toolbar-source-btn",
                        title: "Sources de données pour cette sous-catégorie",
                        onclick: move |_| show_sources_modal.set(true),
                        "🔌 Sources"
                    }
                }
            }

            if !is_specific_subcategory {
                p { class: "toolbar-hint",
                    "Sélectionne une sous-catégorie précise pour ajouter un élément ou configurer une source."
                }
            }

            div { class: "item-grid",
                if is_specific_subcategory {
                    div {
                        class: "item-card item-card--add",
                        onclick: move |_| show_new_item_modal.set(true),
                        "+"
                    }
                }
                for item in visible_items {
                    ItemCard {
                        key: "{item.id}",
                        item: item.clone(),
                        origin_category_id: category_id,
                        origin_subcategory_id: subcategory_id,
                    }
                }
            }

            if has_more {
                div {
                    class: "infinite-scroll-sentinel",
                    onvisible: move |_| {
                        let next_value = *visible_count.read() + BATCH_SIZE;
                        visible_count.set(next_value);
                    },
                    "Chargement..."
                }
            }

            if *show_new_item_modal.read() {
                NewItemModal {
                    category_id,
                    subcategory_id,
                    onclose: move |_| show_new_item_modal.set(false),
                }
            }

            if *show_sources_modal.read() {
                ManageSourcesModal {
                    category_id,
                    subcategory_id,
                    oncancel: move |_| show_sources_modal.set(false),
                }
            }
        }
    }
}