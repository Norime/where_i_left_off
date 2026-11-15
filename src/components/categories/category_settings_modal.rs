use dioxus::prelude::*;
use std::collections::HashMap;

use crate::components::{DeleteButton, LanguageFlag, Modal};
use crate::i18n::{use_texts, Language};
use crate::models::SubcategoryTemplate;
use crate::state::{CategoryStore, ItemStore, SourceStore, TypeStore};

/// Configuration d'une catégorie (`subcategory_id: None`) ou d'une de ses
/// sous-catégories : nom par défaut, nom propre à chaque langue de
/// l'interface, et suppression (avec confirmation).
#[component]
pub fn CategorySettingsModal(category_id: usize, subcategory_id: Option<usize>, onclose: EventHandler<()>) -> Element {
    let i18n_text = use_texts();
    let mut category_store = use_context::<Signal<CategoryStore>>();
    let mut item_store = use_context::<Signal<ItemStore>>();
    let mut type_store = use_context::<Signal<TypeStore>>();
    let mut source_store = use_context::<Signal<SourceStore>>();

    // Valeurs actuelles, lues une seule fois pour pré-remplir le formulaire.
    let (initial_name, initial_translations) = {
        let store = category_store.peek();
        let category = store.categories.iter().find(|c| c.id == category_id);
        match subcategory_id {
            None => category.map(|c| (c.name.clone(), c.translations.clone())),
            Some(sub_id) => category
                .and_then(|c| c.subcategories.iter().find(|s| s.id == sub_id))
                .map(|s| (s.name.clone(), s.translations.clone())),
        }
        .unwrap_or_default()
    };
    let mut name = use_signal(|| initial_name);
    // Modèle : seulement pour une vraie sous-catégorie (pas « All », id 0).
    let has_template = subcategory_id.is_some_and(|sub_id| sub_id != 0);
    let mut template =
        use_signal(|| subcategory_id.and_then(|sub_id| category_store.peek().subcategory_template(category_id, sub_id)));
    // Liste (et non map) pour garder les langues dans l'ordre où on les
    // ajoute ; les traductions existantes suivent l'ordre de `Language::ALL`.
    let mut translations = use_signal(|| {
        Language::ALL
            .iter()
            .filter_map(|language| initial_translations.get(language).map(|t| (*language, t.clone())))
            .collect::<Vec<_>>()
    });
    let mut adding_language = use_signal(|| false);

    let missing_languages: Vec<Language> = Language::ALL
        .iter()
        .copied()
        .filter(|language| !translations.read().iter().any(|(l, _)| l == language))
        .collect();

    let title = if subcategory_id.is_some() {
        i18n_text.subcategory_settings_title
    } else {
        i18n_text.category_settings_title
    };

    let save = move |event: FormEvent| {
        event.prevent_default();
        let new_name = name();
        let new_translations: HashMap<Language, String> = translations().into_iter().collect();
        match subcategory_id {
            None => category_store.write().update_category(category_id, new_name, new_translations),
            Some(sub_id) => {
                let mut store = category_store.write();
                store.update_subcategory(category_id, sub_id, new_name, new_translations);
                store.set_subcategory_template(category_id, sub_id, template());
            }
        }
        onclose.call(());
    };

    let delete = move |_| {
        match subcategory_id {
            None => {
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
            }
            Some(sub_id) => {
                category_store.write().remove_subcategory(category_id, sub_id);
                item_store.write().remove_placements_by_subcategory(category_id, sub_id);
                source_store.write().remove_scopes_by_subcategory(category_id, sub_id);
            }
        }
        onclose.call(());
    };

    rsx! {
        Modal { onclose: move |_| onclose.call(()), class: "category-settings",
            h2 { "{title}" }
            form {
                class: "new-item-modal-form",
                onsubmit: save,
                label {
                    "{i18n_text.name_label}"
                    input {
                        value: "{name}",
                        oninput: move |e| name.set(e.value()),
                    }
                }
                if has_template {
                    label {
                        "{i18n_text.template_label}"
                        select {
                            onchange: move |e| template.set(SubcategoryTemplate::from_key(&e.value())),
                            option { value: "", selected: template().is_none(), "{i18n_text.template_none}" }
                            for option_template in SubcategoryTemplate::ALL {
                                option {
                                    key: "{option_template.key()}",
                                    value: "{option_template.key()}",
                                    selected: template() == Some(option_template),
                                    "{i18n_text.template_name(option_template)}"
                                }
                            }
                        }
                        span { class: "translation-hint", "{i18n_text.template_change_hint}" }
                    }
                }
                fieldset { class: "translation-list",
                    legend { "{i18n_text.translated_names}" }
                    p { class: "translation-hint", "{i18n_text.translated_names_hint}" }
                    for (index , (language , value)) in translations().into_iter().enumerate() {
                        div { key: "{language.code()}", class: "translation-row",
                            LanguageFlag { language }
                            span { class: "translation-language", "{language.native_name()}" }
                            input {
                                placeholder: "{name}",
                                value: "{value}",
                                oninput: move |e| {
                                    if let Some(entry) = translations.write().get_mut(index) {
                                        entry.1 = e.value();
                                    }
                                },
                            }
                            button {
                                r#type: "button",
                                class: "translation-remove",
                                title: "{i18n_text.delete}",
                                onclick: move |_| {
                                    translations.write().remove(index);
                                },
                                "✕"
                            }
                        }
                    }
                    // Seules les langues pas encore ajoutées sont proposées ;
                    // le bouton disparaît quand elles le sont toutes.
                    if !missing_languages.is_empty() {
                        div { class: "language-select",
                            button {
                                r#type: "button",
                                class: "translation-add",
                                onclick: move |_| adding_language.set(!adding_language()),
                                "{i18n_text.add_language}"
                            }
                            if adding_language() {
                                div {
                                    class: "language-select-backdrop",
                                    onclick: move |_| adding_language.set(false),
                                }
                                ul { class: "language-select-menu",
                                    for language in missing_languages {
                                        li { key: "{language.code()}",
                                            button {
                                                r#type: "button",
                                                class: "language-select-option",
                                                onclick: move |_| {
                                                    adding_language.set(false);
                                                    translations.write().push((language, String::new()));
                                                },
                                                LanguageFlag { language }
                                                span { "{language.native_name()}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                div { class: "modal-actions",
                    div { class: "category-settings-delete",
                        DeleteButton { onconfirm: delete }
                    }
                    button {
                        r#type: "button",
                        onclick: move |_| onclose.call(()),
                        "{i18n_text.cancel}"
                    }
                    button { r#type: "submit", class: "primary-btn", "{i18n_text.save}" }
                }
            }
        }
    }
}
