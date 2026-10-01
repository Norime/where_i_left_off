use dioxus::prelude::*;
use std::collections::HashSet;

use crate::components::{DeleteButton, InlineAddForm};
use crate::models::Category;
use crate::routes::Route;
use crate::state::{CategoryStore, ItemStore, SourceStore, TypeStore};

#[component]
pub fn CategoryTreeItem(category: Category, expanded: Signal<HashSet<usize>>) -> Element {
    let mut category_store = use_context::<Signal<CategoryStore>>();
    let mut item_store = use_context::<Signal<ItemStore>>();
    let mut type_store = use_context::<Signal<TypeStore>>();
    let mut source_store = use_context::<Signal<SourceStore>>();
    let category_id = category.id;
    let is_expanded = expanded.read().contains(&category_id);
    let toggle_icon = if is_expanded { "▼" } else { "▶" };

    rsx! {
        li { class: "category-item",
            div {
                class: "category-header",
                onclick: move |_| {
                    let mut set = expanded.write();
                    if set.contains(&category_id) {
                        set.remove(&category_id);
                    } else {
                        set.insert(category_id);
                    }
                },
                span { class: "category-toggle", "{toggle_icon}" }
                span { class: "category-name", "{category.name}" }
                DeleteButton {
                    onconfirm: move |_| {
                        let removed_type_ids: Vec<usize> = type_store
                            .read()
                            .types
                            .iter()
                            .filter(|t| t.category_id == category_id)
                            .map(|t| t.id)
                            .collect();
                        category_store.write().remove_category(category_id);
                        item_store.write().remove_placements_by_category(category_id);
                        item_store.write().remove_type_references(&removed_type_ids);
                        type_store.write().remove_types_by_category(category_id);
                        source_store.write().remove_scopes_by_category(category_id);
                    },
                }
            }
            if is_expanded {
                ul { class: "subcategory-list",
                    for (sub_id , sub_name , is_all) in category
                        .subcategories
                        .iter()
                        .map(|s| (s.id, s.name.clone(), s.id == 0))
                        .collect::<Vec<_>>()
                    {
                        li { key: "{sub_id}", class: "subcategory-item",
                            Link {
                                to: Route::CategoryView {
                                    category_id,
                                    subcategory_id: sub_id,
                                },
                                "{sub_name}"
                            }
                            if !is_all {
                                DeleteButton {
                                    onconfirm: move |_| {
                                        category_store.write().remove_subcategory(category_id, sub_id);
                                        item_store.write().remove_placements_by_subcategory(category_id, sub_id);
                                        source_store.write().remove_scopes_by_subcategory(category_id, sub_id);
                                    },
                                }
                            }
                        }
                    }
                    li {
                        InlineAddForm {
                            placeholder: "Nouvelle sous-catégorie",
                            onsubmit: move |name: String| {
                                category_store.write().add_subcategory(category_id, name);
                            },
                        }
                    }
                }
            }
        }
    }
}