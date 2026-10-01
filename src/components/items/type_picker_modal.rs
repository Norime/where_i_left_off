use dioxus::prelude::*;
use crate::components::InlineAddForm;
use crate::state::{CategoryStore, ItemStore, TypeStore};

/// Relit à chaud la liste des types du placement (catégorie/sous-catégorie)
/// visé, directement depuis le store. Utilisé par tous les checkbox de
/// cette popup pour éviter de capturer un `Vec<usize>` (pas `Copy`) dans
/// plusieurs closures `move` à la fois.
fn current_ids(item_store: Signal<ItemStore>, item_id: usize, category_id: usize, subcategory_id: usize) -> Vec<usize> {
    item_store
        .read()
        .items
        .iter()
        .find(|i| i.id == item_id)
        .and_then(|item| {
            item.placements
                .iter()
                .find(|p| p.category_id == category_id && p.subcategory_id == subcategory_id)
        })
        .map(|p| p.type_ids.clone())
        .unwrap_or_default()
}

fn toggle_type(
    mut item_store: Signal<ItemStore>,
    item_id: usize,
    category_id: usize,
    subcategory_id: usize,
    type_id: usize,
    is_checked: bool,
) {
    let mut new_ids = current_ids(item_store, item_id, category_id, subcategory_id);
    if is_checked {
        if !new_ids.contains(&type_id) {
            new_ids.push(type_id);
        }
    } else {
        new_ids.retain(|existing| *existing != type_id);
    }
    item_store.write().set_placement_types(item_id, category_id, subcategory_id, new_ids);
}

/// Quand on coche un type venant d'une AUTRE catégorie, on ne relie pas
/// directement son id à l'item : ça laisserait le type "prêté" dans son
/// ancienne catégorie sans jamais rejoindre la nôtre. On crée à la place
/// (ou réutilise, `add_type` dédoublonne déjà par nom) un type du même nom
/// DANS la catégorie de l'item, puis c'est CET id-là qu'on associe. C'est
/// une copie du nom, pas un déplacement : le type d'origine reste inchangé
/// dans son ancienne catégorie, donc rien ne casse pour les autres items
/// qui l'utilisaient déjà là-bas.
fn add_foreign_type_to_own_category(
    mut item_store: Signal<ItemStore>,
    mut type_store: Signal<TypeStore>,
    item_id: usize,
    category_id: usize,
    subcategory_id: usize,
    type_name: String,
) {
    let local_id = type_store.write().add_type(category_id, type_name);
    let mut new_ids = current_ids(item_store, item_id, category_id, subcategory_id);
    if !new_ids.contains(&local_id) {
        new_ids.push(local_id);
    }
    item_store.write().set_placement_types(item_id, category_id, subcategory_id, new_ids);
}

/// Popup de gestion des types d'un placement précis d'un item : une section
/// pour créer un nouveau type (propre à la catégorie de l'item), une grille
/// avec les types de SA catégorie, et une autre avec les types des AUTRES
/// catégories (rien n'empêche techniquement un item d'avoir aussi un type
/// "emprunté" ailleurs, le store ne restreint pas `type_ids` par catégorie).
#[component]
pub fn TypePickerModal(
    item_id: usize,
    category_id: usize,
    subcategory_id: usize,
    onclose: EventHandler<()>,
) -> Element {
    let mut item_store = use_context::<Signal<ItemStore>>();
    let mut type_store = use_context::<Signal<TypeStore>>();
    let category_store = use_context::<Signal<CategoryStore>>();

    let current_type_ids = current_ids(item_store, item_id, category_id, subcategory_id);

    let own_category_name = category_store
        .read()
        .categories
        .iter()
        .find(|c| c.id == category_id)
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "cette catégorie".to_string());

    let own_types: Vec<(usize, String, bool)> = type_store
        .read()
        .types
        .iter()
        .filter(|t| t.category_id == category_id)
        .map(|t| (t.id, t.name.clone(), current_type_ids.contains(&t.id)))
        .collect();

    // Types des autres catégories, regroupés par catégorie pour rester
    // lisible (une section par catégorie, comme "GENRES" / "FORMATS" sur
    // un site comme Comix).
    let other_groups: Vec<(usize, String, Vec<(usize, String, bool)>)> = category_store
        .read()
        .categories
        .iter()
        .filter(|c| c.id != category_id)
        .filter_map(|c| {
            let types: Vec<(usize, String, bool)> = type_store
                .read()
                .types
                .iter()
                .filter(|t| t.category_id == c.id)
                .map(|t| (t.id, t.name.clone(), current_type_ids.contains(&t.id)))
                .collect();
            if types.is_empty() { None } else { Some((c.id, c.name.clone(), types)) }
        })
        .collect();

    rsx! {
        div { class: "type-picker-overlay", onclick: move |_| onclose.call(()),
            div {
                class: "type-picker-panel",
                onclick: move |event| event.stop_propagation(),
                div { class: "type-picker-header",
                    h3 { "Types" }
                    button {
                        r#type: "button",
                        class: "icon-button",
                        onclick: move |_| onclose.call(()),
                        "×"
                    }
                }
                InlineAddForm {
                    placeholder: "Nouveau type pour {own_category_name}",
                    onsubmit: move |name: String| {
                        let new_id = type_store.write().add_type(category_id, name);
                        let mut new_ids = current_ids(item_store, item_id, category_id, subcategory_id);
                        if !new_ids.contains(&new_id) {
                            new_ids.push(new_id);
                        }
                        item_store
                            .write()
                            .set_placement_types(item_id, category_id, subcategory_id, new_ids);
                    },
                }
                div { class: "type-picker-section",
                    h4 { "{own_category_name}" }
                    if own_types.is_empty() {
                        span { class: "toolbar-hint", "Aucun type créé pour cette catégorie" }
                    } else {
                        div { class: "type-picker-grid",
                            for (type_id , type_name , checked) in own_types {
                                label {
                                    class: "type-filter-item",
                                    key: "{type_id}",
                                    input {
                                        r#type: "checkbox",
                                        checked,
                                        oninput: move |event| {
                                            let is_checked = event.value().parse().unwrap_or(false);
                                            toggle_type(
                                                item_store,
                                                item_id,
                                                category_id,
                                                subcategory_id,
                                                type_id,
                                                is_checked,
                                            );
                                        },
                                    }
                                    "{type_name}"
                                }
                            }
                        }
                    }
                }
                if !other_groups.is_empty() {
                    div { class: "type-picker-section",
                        h4 { "Autres catégories" }
                        for (group_cat_id , group_name , types) in other_groups {
                            div {
                                class: "type-picker-subgroup",
                                key: "{group_cat_id}",
                                span { class: "type-picker-subgroup-title", "{group_name}" }
                                div { class: "type-picker-grid",
                                    for (type_id , type_name , checked) in types {
                                        label {
                                            class: "type-filter-item",
                                            key: "{type_id}",
                                            input {
                                                r#type: "checkbox",
                                                checked,
                                                oninput: move |event| {
                                                    let is_checked = event.value().parse().unwrap_or(false);
                                                    if is_checked {
                                                        add_foreign_type_to_own_category(
                                                            item_store,
                                                            type_store,
                                                            item_id,
                                                            category_id,
                                                            subcategory_id,
                                                            type_name.clone(),
                                                        );
                                                    } else {
                                                        toggle_type(
                                                            item_store,
                                                            item_id,
                                                            category_id,
                                                            subcategory_id,
                                                            type_id,
                                                            false,
                                                        );
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
                }
            }
        }
    }
}