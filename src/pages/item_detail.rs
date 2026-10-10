use dioxus::prelude::*;
use crate::components::{AddPlacementModal, DeleteButton, EditItemModal, TypePickerModal, ItemTabs};
use crate::i18n::use_texts;
use crate::markdown;
use crate::models::{ProgressNodeKind, WorkStatus};
use crate::state::{CategoryStore, ItemStore, TypeStore};

/// Affiche le détail d'un élément, avec un panneau "Catégories" dans la
/// colonne de droite, et le statut + le type + la progression du
/// placement sélectionné dans le bloc du bas (le type et la progression
/// sont propres à chaque catégorie/sous-catégorie, donc affichés là et pas
/// globalement pour l'item).
#[component]
pub fn ItemDetail(id: usize, category_id: usize, subcategory_id: usize) -> Element {
    let i18n_text = use_texts();
    let mut item_store = use_context::<Signal<ItemStore>>();
    let category_store = use_context::<Signal<CategoryStore>>();
    let type_store = use_context::<Signal<TypeStore>>();
    let mut show_edit = use_signal(|| false);
    let mut show_add_placement = use_signal(|| false);
    let mut show_type_picker = use_signal(|| false);
    let mut placement_error = use_signal(|| false);
    let mut selected_placement = use_signal(|| None::<(usize, usize)>);
    let mut description_expanded = use_signal(|| false);

    let item = item_store.read().items.iter().find(|i| i.id == id).cloned();

    match item {
        Some(item) => {
            let image_style = item.image_url
                .as_ref()
                .map(|url| format!("background-image: url('{url}');"))
                .unwrap_or_default();

            let description_html = markdown::to_html(&item.description);
            let description_class = if description_expanded() {
                "item-detail-description item-detail-description--expanded"
            } else {
                "item-detail-description"
            };
            let toggle_label = if description_expanded() { i18n_text.show_less } else { i18n_text.show_more };

            let placements: Vec<(usize, usize, String, &'static str)> = item.placements.iter()
                .map(|p| {
                    let category = category_store.read().categories.iter()
                        .find(|c| c.id == p.category_id)
                        .cloned();
                    let category_name = category.as_ref().map(|c| c.name.clone()).unwrap_or_else(|| "?".to_string());
                    let subcategory_name = category.as_ref()
                        .and_then(|c| c.subcategories.iter().find(|s| s.id == p.subcategory_id).cloned())
                        .map(|s| s.name)
                        .unwrap_or_else(|| "?".to_string());
                    let label = format!("{category_name} › {subcategory_name}");
                    (p.category_id, p.subcategory_id, label, p.status.value_key())
                })
                .collect();

            // Si on arrive depuis une catégorie/sous-catégorie précise
            // (via `category_id`/`subcategory_id` passés par `ItemCard`
            // dans l'URL), on rouvre directement sur CE placement — à
            // condition qu'il existe bien sur l'item. Sinon on retombe sur
            // le choix manuel (`selected_placement`) puis sur le premier
            // placement de l'item.
            let effective_placement = selected_placement()
                .or_else(|| {
                    if category_id != 0
                        && subcategory_id != 0
                        && item.placements.iter().any(|p| {
                            p.category_id == category_id && p.subcategory_id == subcategory_id
                        })
                    {
                        Some((category_id, subcategory_id))
                    } else {
                        None
                    }
                })
                .or_else(|| item.placements.first().map(|p| (p.category_id, p.subcategory_id)));

            let selected_placement_value = effective_placement
                .map(|(c, s)| format!("{c}:{s}"))
                .unwrap_or_default();

            let current_status_key: &'static str = effective_placement
                .and_then(|(cat_id, sub_id)| placements.iter().find(|(c, s, _, _)| *c == cat_id && *s == sub_id))
                .map(|(_, _, _, key)| *key)
                .unwrap_or("new");

            let current_status_label = match current_status_key {
                "in_progress" => i18n_text.status_in_progress,
                "done" => i18n_text.status_done,
                _ => i18n_text.status_new,
            };
            // Statut de L'ŒUVRE (en cours / terminé / non fini) : propre à
            // CE placement, pas à l'item entier — un anime peut être
            // abandonné pendant que le manga du même item continue.
            let current_work_status_key: &'static str = effective_placement
                .and_then(|(cat_id, sub_id)| item.placements.iter().find(|p| p.category_id == cat_id && p.subcategory_id == sub_id))
                .map(|p| p.work_status.value_key())
                .unwrap_or("in_progress");

            // Le type est propre au placement sélectionné (une catégorie a
            // ses propres types) : on retrouve la liste de ce placement
            // précis, puis on construit les lignes de cases à cocher à
            // partir des types de SA catégorie uniquement.
            let current_type_ids: Vec<usize> = effective_placement
                .and_then(|(cat_id, sub_id)| item.placements.iter().find(|p| p.category_id == cat_id && p.subcategory_id == sub_id))
                .map(|p| p.type_ids.clone())
                .unwrap_or_default();
            let current_type_names: Vec<String> = current_type_ids
                .iter()
                .filter_map(|tid| type_store.read().types.iter().find(|ty| ty.id == *tid).map(|ty| ty.name.clone()))
                .collect();
            let current_type_display = if current_type_names.is_empty() {
                i18n_text.no_type.to_string()
            } else {
                current_type_names.join(", ")
            };

            // Le statut se calcule tout seul dès qu'il y a de la
            // progression à suivre (voir `recompute_placement_status`
            // côté store) ; il est seulement affiché ici, sans possibilité
            // de le modifier à la main.
            let has_progress: bool = effective_placement
                .and_then(|(cat_id, sub_id)| item.placements.iter().find(|p| p.category_id == cat_id && p.subcategory_id == sub_id))
                .map(|p| p.progress.iter().any(|n| n.kind != ProgressNodeKind::Group))
                .unwrap_or(false);

            rsx! {
                div { class: "item-detail",
                    div { class: "item-detail-hero", style: "{image_style}",
                        button {
                            class: "back-button",
                            onclick: move |_| {
                                navigator().go_back();
                            },
                            "←"
                        }
                        div { class: "item-detail-hero-actions",
                            button {
                                class: "icon-button",
                                onclick: move |_| show_edit.set(true),
                                "⚙"
                            }
                            DeleteButton {
                                onconfirm: move |_| {
                                    item_store.write().remove_item(id);
                                    navigator().go_back();
                                },
                            }
                        }
                        h1 { class: "item-detail-title", "{item.title}" }
                    }
                    div { class: "item-detail-side",
                        div {
                            class: "item-detail-poster",
                            style: "{image_style}",
                        }
                        div { class: "item-detail-links",
                            div { class: "item-detail-links-header",
                                h3 { "{i18n_text.categories}" }
                                button {
                                    class: "add-placement-btn",
                                    onclick: move |_| show_add_placement.set(true),
                                    "+"
                                }
                            }
                            div { class: "placement-chip-list",
                                for (cat_id , sub_id , label , _status_key) in placements.iter().cloned() {
                                    span {
                                        class: "placement-chip",
                                        key: "{cat_id}-{sub_id}",
                                        "{label}"
                                        button {
                                            r#type: "button",
                                            class: "placement-chip-remove",
                                            onclick: move |_| {
                                                match item_store.write().remove_placement(id, cat_id, sub_id) {
                                                    Ok(()) => placement_error.set(false),
                                                    Err(_) => placement_error.set(true),
                                                }
                                            },
                                            "×"
                                        }
                                    }
                                }
                            }
                            if placement_error() {
                                p { class: "auth-error", "{i18n_text.last_placement_error}" }
                            }
                        }
                    }
                    div { class: "item-detail-content",
                        div {
                            class: "{description_class}",
                            dangerous_inner_html: "{description_html}",
                        }
                        button {
                            class: "description-toggle-btn",
                            onclick: move |_| description_expanded.set(!description_expanded()),
                            "{toggle_label}"
                        }

                        div { class: "placement-detail-panel",
                            div { class: "placement-detail-header",
                                select {
                                    class: "placement-detail-select",
                                    value: "{selected_placement_value}",
                                    onchange: move |event| {
                                        let value = event.value();
                                        let mut parts = value.split(':');
                                        let cat = parts.next().and_then(|s| s.parse::<usize>().ok());
                                        let sub = parts.next().and_then(|s| s.parse::<usize>().ok());
                                        if let (Some(cat), Some(sub)) = (cat, sub) {
                                            selected_placement.set(Some((cat, sub)));
                                        }
                                    },
                                    for (cat_id , sub_id , label , _key) in placements.iter().cloned() {
                                        option {
                                            key: "{cat_id}-{sub_id}",
                                            value: "{cat_id}:{sub_id}",
                                            selected: effective_placement == Some((cat_id, sub_id)),
                                            "{label}"
                                        }
                                    }
                                }
                            }
                            if let Some((cat_id, sub_id)) = effective_placement {
                                div { class: "placement-detail-fields",
                                    label { class: "field-row",
                                        span { class: "field-label", "{i18n_text.work_status_label}" }
                                        select {
                                            value: "{current_work_status_key}",
                                            onchange: move |event| {
                                                let new_status = match event.value().as_str() {
                                                    "completed" => WorkStatus::Completed,
                                                    "unfinished" => WorkStatus::Unfinished,
                                                    _ => WorkStatus::InProgress,
                                                };
                                                item_store.write().set_placement_work_status(id, cat_id, sub_id, new_status);
                                            },
                                            option { value: "in_progress", "{i18n_text.work_status_in_progress}" }
                                            option { value: "completed", "{i18n_text.work_status_completed}" }
                                            option { value: "unfinished", "{i18n_text.work_status_unfinished}" }
                                        }
                                    }
                                    div { class: "field-row",
                                        span { class: "field-label", "{i18n_text.status_label}" }
                                        span { class: "status-display", "{current_status_label}" }
                                        if has_progress {
                                            span { class: "progress-usage-hint",
                                                "{i18n_text.computed_from_progress}"
                                            }
                                        }
                                    }
                                    div { class: "field-row",
                                        span { class: "field-label", "{i18n_text.type_label}" }
                                        span { class: "type-display-list", "{current_type_display}" }
                                        button {
                                            r#type: "button",
                                            class: "icon-button",
                                            onclick: move |_| show_type_picker.set(true),
                                            "✎"
                                        }
                                    }
                                    ItemTabs {
                                        item_id: item.id,
                                        category_id: cat_id,
                                        subcategory_id: sub_id,
                                    }
                                }
                            }
                        }
                        if show_type_picker() {
                            if let Some((cat_id, sub_id)) = effective_placement {
                                TypePickerModal {
                                    item_id: id,
                                    category_id: cat_id,
                                    subcategory_id: sub_id,
                                    onclose: move |_| show_type_picker.set(false),
                                }
                            }
                        }
                    }
                }
                if show_edit() {
                    EditItemModal {
                        item: item.clone(),
                        onclose: move |_| show_edit.set(false),
                    }
                }
                if show_add_placement() {
                    AddPlacementModal {
                        item_id: id,
                        onclose: move |_| show_add_placement.set(false),
                    }
                }
            }
        }
        None => rsx! {
            p { "{i18n_text.item_not_found}" }
        },
    }
}