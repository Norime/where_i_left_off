use dioxus::prelude::*;
use std::collections::HashSet;

use crate::components::{CategorySettingsModal, InlineAddForm};
use crate::i18n::{use_texts, Language};
use crate::models::Category;
use crate::routes::Route;
use crate::state::CategoryStore;

#[component]
pub fn CategoryTreeItem(category: Category, expanded: Signal<HashSet<usize>>) -> Element {
    let i18n_text = use_texts();
    let language = use_context::<Memo<Language>>()();
    let mut category_store = use_context::<Signal<CategoryStore>>();
    // Élément en cours de configuration : `Some(None)` pour la catégorie,
    // `Some(Some(id))` pour une de ses sous-catégories.
    let mut editing = use_signal(|| None::<Option<usize>>);
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
                span { class: "category-name", "{category.display_name(language)}" }
                button {
                    class: "settings-btn",
                    title: "{i18n_text.settings}",
                    // Sans `stop_propagation`, le clic replierait/déplierait la catégorie.
                    onclick: move |evt| {
                        evt.stop_propagation();
                        editing.set(Some(None));
                    },
                    "⚙"
                }
            }
            if is_expanded {
                ul { class: "subcategory-list",
                    for (sub_id , sub_name , is_all) in category
                        .subcategories
                        .iter()
                        .map(|s| (s.id, s.display_name(language).to_string(), s.id == 0))
                        .collect::<Vec<_>>()
                    {
                        li { key: "{sub_id}", class: "subcategory-item",
                            Link {
                                to: Route::CategoryView {
                                    category_id,
                                    subcategory_id: sub_id,
                                },
                                // La sous-catégorie "toutes" est stockée sous le nom "All" :
                                // on affiche son nom dans la langue active.
                                if is_all { "{i18n_text.subcategory_all}" } else { "{sub_name}" }
                            }
                            if !is_all {
                                button {
                                    class: "settings-btn",
                                    title: "{i18n_text.settings}",
                                    onclick: move |_| editing.set(Some(Some(sub_id))),
                                    "⚙"
                                }
                            }
                        }
                    }
                    li {
                        InlineAddForm {
                            placeholder: "{i18n_text.new_subcategory}",
                            onsubmit: move |name: String| {
                                category_store.write().add_subcategory(category_id, name);
                            },
                        }
                    }
                }
            }
            // Rendue hors de `.category-header` pour que les clics dans la
            // modale ne remontent pas jusqu'au repli de la catégorie.
            if let Some(subcategory_id) = editing() {
                CategorySettingsModal {
                    category_id,
                    subcategory_id,
                    onclose: move |_| editing.set(None),
                }
            }
        }
    }
}
