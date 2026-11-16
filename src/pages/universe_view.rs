use dioxus::prelude::*;

use crate::components::DeleteButton;
use crate::i18n::{use_texts, Language, Texts};
use crate::models::{entry_placement, item_release_label, Category, ItemStatus, UniverseOrderKind};
use crate::routes::Route;
use crate::state::{CategoryStore, ItemStore, UniverseStore};

/// Nombre de fiches proposées sous la recherche d'ajout.
const MAX_SEARCH_RESULTS: usize = 8;

/// « Catégorie › Sous-catégorie », comme sur la fiche.
pub fn placement_label(categories: &[Category], i18n_text: &Texts, language: Language, category_id: usize, subcategory_id: usize) -> String {
    let category = categories.iter().find(|c| c.id == category_id);
    let category_name = category.map(|c| c.display_name(language).to_string()).unwrap_or_else(|| "?".to_string());
    let subcategory_name = match subcategory_id {
        0 => i18n_text.subcategory_all.to_string(),
        _ => category
            .and_then(|c| c.subcategories.iter().find(|s| s.id == subcategory_id))
            .map(|s| s.display_name(language).to_string())
            .unwrap_or_else(|| "?".to_string()),
    };
    format!("{category_name} › {subcategory_name}")
}

/// Une œuvre de l'univers, prête à afficher dans l'ordre choisi.
#[derive(Clone, PartialEq)]
struct UniverseRow {
    entry_id: usize,
    item_id: usize,
    category_id: usize,
    subcategory_id: usize,
    title: String,
    image_url: Option<String>,
    placement: String,
    release: Option<String>,
    status: ItemStatus,
    status_label: &'static str,
}

/// Page d'un univers : choix de l'ordre (sortie, chronologique…),
/// progression et prochain à voir, liste des œuvres, ajout par recherche.
#[component]
pub fn UniverseView(id: usize) -> Element {
    let i18n_text = use_texts();
    let language = use_context::<Memo<Language>>()();
    let mut universe_store = use_context::<Signal<UniverseStore>>();
    let item_store = use_context::<Signal<ItemStore>>();
    let category_store = use_context::<Signal<CategoryStore>>();
    let mut query = use_signal(String::new);
    let mut new_order_name = use_signal(String::new);
    let mut new_order_manual = use_signal(|| true);

    let Some(universe) = universe_store.read().universe(id).cloned() else {
        return rsx! {
            div { class: "universe-view",
                p { class: "empty-state", "{i18n_text.universe_not_found}" }
            }
        };
    };
    let items = item_store.read().items.clone();
    let categories = category_store.read().categories.clone();
    let order = universe.current_order().cloned();
    let order_id = order.as_ref().map(|o| o.id).unwrap_or(0);
    let is_manual = matches!(order.as_ref().map(|o| &o.kind), Some(UniverseOrderKind::Manual(_)));

    let rows: Vec<UniverseRow> = order
        .as_ref()
        .map(|order| universe.ordered_entries(order, &items))
        .unwrap_or_default()
        .iter()
        .filter_map(|entry| {
            let (item, placement) = entry_placement(entry, &items)?;
            let done_word = category_store
                .read()
                .subcategory_template(entry.category_id, entry.subcategory_id)
                .map(|t| t.done_word())
                .unwrap_or_default();
            Some(UniverseRow {
                entry_id: entry.id,
                item_id: item.id,
                category_id: entry.category_id,
                subcategory_id: entry.subcategory_id,
                title: item.title.clone(),
                image_url: item.image_url.clone(),
                placement: placement_label(&categories, i18n_text, language, entry.category_id, entry.subcategory_id),
                release: item_release_label(&item.extra_info).map(str::to_string),
                status: placement.status,
                status_label: i18n_text.status_name_in(placement.status, done_word),
            })
        })
        .collect();
    let progress = universe.progress(&items);
    let (done, total) = (progress.done, progress.total);
    let next = progress.next.and_then(|entry| rows.iter().find(|r| r.entry_id == entry.id).cloned());
    let percent = if total > 0 { done * 100 / total } else { 0 };

    // Recherche d'ajout : fiches dont le titre contient le texte, un
    // résultat par placement pas encore dans l'univers.
    let search = query.read().trim().to_lowercase();
    let results: Vec<(usize, usize, usize, String, String)> = if search.is_empty() {
        Vec::new()
    } else {
        items
            .iter()
            .filter(|item| item.title.to_lowercase().contains(&search))
            .flat_map(|item| {
                item.placements
                    .iter()
                    .filter(|p| !universe.contains(item.id, p.category_id, p.subcategory_id))
                    .map(|p| {
                        let label = placement_label(&categories, i18n_text, language, p.category_id, p.subcategory_id);
                        (item.id, p.category_id, p.subcategory_id, item.title.clone(), label)
                    })
            })
            .take(MAX_SEARCH_RESULTS)
            .collect()
    };

    let mut add_order = move || {
        let name = new_order_name.read().trim().to_string();
        if name.is_empty() {
            return;
        }
        let items = item_store.read().items.clone();
        universe_store.write().add_order(id, name, new_order_manual(), &items);
        new_order_name.set(String::new());
    };

    rsx! {
        div { class: "universe-view",
            div { class: "universe-header",
                input {
                    r#type: "text",
                    class: "universe-name",
                    value: "{universe.name}",
                    title: "{i18n_text.universe_name}",
                    onchange: move |event| universe_store.write().rename_universe(id, event.value()),
                }
                DeleteButton {
                    onconfirm: move |_| {
                        universe_store.write().remove_universe(id);
                        navigator().push(Route::Home {});
                    },
                }
            }

            // Ordres : un onglet chacun, puis l'ajout d'un ordre.
            div { class: "universe-orders",
                for option in universe.orders.iter().cloned() {
                    button {
                        key: "{option.id}",
                        r#type: "button",
                        class: if option.id == order_id { "home-tab active" } else { "home-tab" },
                        onclick: move |_| universe_store.write().select_order(id, option.id),
                        "{option.name}"
                    }
                }
                div { class: "universe-order-add",
                    input {
                        r#type: "text",
                        placeholder: "{i18n_text.universe_new_order}",
                        value: "{new_order_name}",
                        oninput: move |event| new_order_name.set(event.value()),
                        onkeydown: move |event| {
                            if event.key() == Key::Enter {
                                add_order();
                            }
                        },
                    }
                    select {
                        onchange: move |event| new_order_manual.set(event.value() == "manual"),
                        option { value: "manual", selected: new_order_manual(), "{i18n_text.universe_order_manual_kind}" }
                        option { value: "release", selected: !new_order_manual(), "{i18n_text.universe_order_release_kind}" }
                    }
                    button { r#type: "button", class: "secondary-btn", onclick: move |_| add_order(), "+" }
                }
            }
            if let Some(order) = order.clone() {
                div { class: "universe-order-info",
                    input {
                        r#type: "text",
                        class: "universe-order-name",
                        value: "{order.name}",
                        onchange: move |event| universe_store.write().rename_order(id, order.id, event.value()),
                    }
                    span { class: "progress-usage-hint",
                        if is_manual {
                            "{i18n_text.universe_manual_hint}"
                        } else {
                            "{i18n_text.universe_release_hint}"
                        }
                    }
                    if universe.orders.len() > 1 {
                        button {
                            r#type: "button",
                            class: "universe-order-delete",
                            onclick: move |_| universe_store.write().remove_order(id, order.id),
                            "{i18n_text.universe_delete_order}"
                        }
                    }
                }
            }

            if total > 0 {
                div { class: "universe-summary",
                    div { class: "home-check-bar universe-summary-bar",
                        div { class: "home-check-bar-fill", width: "{percent}%" }
                    }
                    span { class: "universe-summary-count", {(i18n_text.progress_count)(done, total)} }
                    match &next {
                        Some(next) => rsx! {
                            Link {
                                class: "universe-next",
                                to: Route::ItemDetail {
                                    id: next.item_id,
                                    category_id: next.category_id,
                                    subcategory_id: next.subcategory_id,
                                },
                                {(i18n_text.universe_next)(&next.title)}
                            }
                        },
                        None => rsx! {
                            span { class: "universe-next", "{i18n_text.universe_all_done}" }
                        },
                    }
                }
            } else {
                p { class: "empty-state", "{i18n_text.universe_empty}" }
            }

            ol { class: "universe-list",
                for (index , row) in rows.into_iter().enumerate() {
                    li {
                        key: "{row.entry_id}",
                        class: if next.as_ref().is_some_and(|n| n.entry_id == row.entry_id) { "universe-row universe-row--next" } else { "universe-row" },
                        span { class: "universe-row-number", "{index + 1}" }
                        div {
                            class: "universe-row-image",
                            style: row.image_url.as_ref().map(|url| format!("background-image: url('{url}');")).unwrap_or_default(),
                        }
                        div { class: "universe-row-body",
                            Link {
                                class: "universe-row-title",
                                to: Route::ItemDetail {
                                    id: row.item_id,
                                    category_id: row.category_id,
                                    subcategory_id: row.subcategory_id,
                                },
                                "{row.title}"
                            }
                            span { class: "universe-row-meta",
                                "{row.placement} · "
                                {row.release.clone().unwrap_or_else(|| i18n_text.universe_no_date.to_string())}
                            }
                        }
                        span { class: "universe-status universe-status--{row.status.value_key()}", "{row.status_label}" }
                        if is_manual {
                            button {
                                r#type: "button",
                                class: "universe-move",
                                title: "{i18n_text.move_up}",
                                disabled: index == 0,
                                onclick: move |_| {
                                    let items = item_store.read().items.clone();
                                    universe_store.write().move_entry(id, order_id, row.entry_id, -1, &items);
                                },
                                "↑"
                            }
                            button {
                                r#type: "button",
                                class: "universe-move",
                                title: "{i18n_text.move_down}",
                                disabled: index + 1 == total,
                                onclick: move |_| {
                                    let items = item_store.read().items.clone();
                                    universe_store.write().move_entry(id, order_id, row.entry_id, 1, &items);
                                },
                                "↓"
                            }
                        }
                        button {
                            r#type: "button",
                            class: "universe-remove",
                            title: "{i18n_text.universe_remove_entry}",
                            onclick: move |_| universe_store.write().remove_entry(id, row.entry_id),
                            "×"
                        }
                    }
                }
            }

            div { class: "universe-add",
                input {
                    r#type: "text",
                    class: "search-input",
                    placeholder: "{i18n_text.universe_add_search}",
                    value: "{query}",
                    oninput: move |event| query.set(event.value()),
                }
                if !search.is_empty() {
                    if results.is_empty() {
                        p { class: "progress-usage-hint", "{i18n_text.universe_no_match}" }
                    }
                    ul { class: "universe-add-results",
                        for (item_id , category_id , subcategory_id , title , label) in results {
                            li { key: "{item_id}-{category_id}-{subcategory_id}",
                                button {
                                    r#type: "button",
                                    onclick: move |_| universe_store.write().add_entry(id, item_id, category_id, subcategory_id),
                                    strong { "+ {title}" }
                                    span { class: "universe-row-meta", " · {label}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
