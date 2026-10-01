use dioxus::prelude::*;
use std::collections::HashSet;
use crate::state::TypeStore;

/// Liste de cases à cocher pour choisir un ou plusieurs types, limitée aux
/// types des catégories données. `selected` est un signal partagé avec
/// l'appelant : cocher/décocher ici met à jour son état directement.
#[component]
pub fn TypeCheckboxList(category_ids: Vec<usize>, selected: Signal<HashSet<usize>>) -> Element {
    let type_store = use_context::<Signal<TypeStore>>();

    let type_checkbox_state: Vec<(usize, String, bool)> = type_store.read().types.iter()
        .filter(|t| category_ids.contains(&t.category_id))
        .map(|t| (t.id, t.name.clone(), selected.read().contains(&t.id)))
        .collect();

    rsx! {
        if type_checkbox_state.is_empty() {
            span { class: "toolbar-hint", "Aucun type créé pour cette catégorie" }
        } else {
            div { class: "type-filter-list",
                for (type_id , type_name , checked) in type_checkbox_state {
                    label { class: "type-filter-item", key: "{type_id}",
                        input {
                            r#type: "checkbox",
                            checked,
                            oninput: move |event| {
                                let is_checked = event.value().parse().unwrap_or(false);
                                let mut set = selected.write();
                                if is_checked {
                                    set.insert(type_id);
                                } else {
                                    set.remove(&type_id);
                                }
                            },
                        }
                        "{type_name}"
                    }
                }
            }
        }
    }
}