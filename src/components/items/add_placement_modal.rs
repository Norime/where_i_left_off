use dioxus::prelude::*;
use crate::components::{FetchedItemData, ItemSourceSearch, Modal};
use crate::i18n::{use_texts, Language};
use crate::models::{ImportedFrom, ProgressNode, ProgressNodeKind};
use crate::state::{CategoryStore, ItemStore};

/// Fenêtre pour rattacher un élément à une catégorie/sous-catégorie
/// supplémentaire : une liste déroulante de catégories (uniquement celles
/// qui ont au moins une vraie sous-catégorie), puis une seconde liste avec
/// les sous-catégories de la catégorie choisie. Si des sources sont liées à
/// la sous-catégorie choisie, on peut aussi y chercher l'œuvre pour importer
/// sa liste (épisodes, chapitres...) et la relier à cette source.
#[component]
pub fn AddPlacementModal(item_id: usize, onclose: EventHandler<()>) -> Element {
    let i18n_text = use_texts();
    let language = use_context::<Memo<Language>>()();
    let mut item_store = use_context::<Signal<ItemStore>>();
    let category_store = use_context::<Signal<CategoryStore>>();

    let mut selected_category = use_signal(|| None::<usize>);
    let mut selected_subcategory = use_signal(|| None::<usize>);
    let mut error = use_signal(|| None::<&'static str>);
    let mut progress = use_signal(Vec::<ProgressNode>::new);
    let mut imported_from = use_signal(|| None::<ImportedFrom>);

    let item_title = item_store.read().items.iter()
        .find(|i| i.id == item_id)
        .map(|i| i.title.clone())
        .unwrap_or_default();
    let imported_count = progress.read().iter().filter(|n| n.kind != ProgressNodeKind::Group).count();

    let available_categories: Vec<(usize, String)> = category_store.read().categories.iter()
        .filter(|c| c.subcategories.iter().any(|s| s.id != 0))
        .map(|c| (c.id, c.display_name(language).to_string()))
        .collect();

    let available_subcategories: Vec<(usize, String)> = match selected_category() {
        Some(cat_id) => category_store.read().categories.iter()
            .find(|c| c.id == cat_id)
            .map(|c| c.subcategories.iter().filter(|s| s.id != 0).map(|s| (s.id, s.display_name(language).to_string())).collect())
            .unwrap_or_default(),
        None => Vec::new(),
    };

    let category_value = selected_category().map(|id| id.to_string()).unwrap_or_default();
    let subcategory_value = selected_subcategory().map(|id| id.to_string()).unwrap_or_default();

    rsx! {
        Modal { onclose: move |_| onclose.call(()),
            h2 { "{i18n_text.add_to_category}" }
            div { class: "placement-picker-form",
                label {
                    "{i18n_text.category}"
                    select {
                        value: "{category_value}",
                        onchange: move |event| {
                            match event.value().parse::<usize>() {
                                Ok(id) => selected_category.set(Some(id)),
                                Err(_) => selected_category.set(None),
                            }
                            selected_subcategory.set(None);
                            progress.set(Vec::new());
                            imported_from.set(None);
                        },
                        option { value: "", "{i18n_text.choose_category}" }
                        for (id , name) in available_categories.iter() {
                            option { key: "{id}", value: "{id}", "{name}" }
                        }
                    }
                }
                label {
                    "{i18n_text.subcategory}"
                    select {
                        value: "{subcategory_value}",
                        disabled: selected_category().is_none(),
                        onchange: move |event| {
                            match event.value().parse::<usize>() {
                                Ok(id) => selected_subcategory.set(Some(id)),
                                Err(_) => selected_subcategory.set(None),
                            }
                            progress.set(Vec::new());
                            imported_from.set(None);
                        },
                        option { value: "", "{i18n_text.choose_subcategory}" }
                        for (id , name) in available_subcategories.iter() {
                            option { key: "{id}", value: "{id}", "{name}" }
                        }
                    }
                }
                if let (Some(cat), Some(sub)) = (selected_category(), selected_subcategory()) {
                    ItemSourceSearch {
                        key: "{cat}-{sub}",
                        scopes: vec![(cat, sub)],
                        import_progress: true,
                        initial_query: item_title.clone(),
                        on_apply: move |data: FetchedItemData| {
                            progress.set(data.progress);
                            imported_from.set(data.imported_from);
                        },
                    }
                    if imported_count > 0 {
                        p { class: "toolbar-hint", {(i18n_text.imported_entries)(imported_count)} }
                    }
                }
                if let Some(message) = error() {
                    p { class: "auth-error", "{message}" }
                }
                div { class: "modal-actions",
                    button { r#type: "button", onclick: move |_| onclose.call(()), "{i18n_text.cancel}" }
                    button {
                        r#type: "button",
                        class: "primary-btn",
                        onclick: move |_| {
                            match (selected_category(), selected_subcategory()) {
                                (Some(cat), Some(sub)) => {
                                    let mut store = item_store.write();
                                    store.add_placement(item_id, cat, sub);
                                    if let Some(from) = imported_from() {
                                        store.set_placement_progress(item_id, cat, sub, progress());
                                        store.set_placement_import(item_id, cat, sub, Some(from));
                                    }
                                    if let Some(template) = category_store.read().subcategory_template(cat, sub) {
                                        store.apply_template(item_id, cat, sub, template, i18n_text);
                                    }
                                    drop(store);
                                    onclose.call(());
                                }
                                _ => error.set(Some(i18n_text.choose_category_and_subcategory)),
                            }
                        },
                        "{i18n_text.add}"
                    }
                }
            }
        }
    }
}