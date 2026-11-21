use dioxus::prelude::*;
use std::collections::HashSet;
use crate::components::{CategoryTreeItem, Modal};
use crate::i18n::use_texts;
use crate::routes::Route;
use crate::state::{CategoryStore, UniverseStore, UserStore};

/// Menu de gauche : compte connecté avec accès aux réglages du profil
/// (roue crantée), arborescence des catégories/sous-catégories avec l'ajout
/// d'une catégorie, puis les univers avec l'ajout d'un univers.
#[component]
pub fn Sidebar() -> Element {
    let i18n_text = use_texts();
    let mut store = use_context::<Signal<CategoryStore>>();
    let session = use_context::<Signal<Option<usize>>>();
    let user_store = use_context::<Signal<UserStore>>();
    let expanded = use_signal(HashSet::<usize>::new);
    let mut universe_store = use_context::<Signal<UniverseStore>>();

    let username = session()
        .and_then(|id| {
            user_store.read().users.iter()
                .find(|u| u.id == id)
                .map(|u| u.username.clone())
        })
        .unwrap_or_default();

    rsx! {
        nav { class: "sidebar",
            div { class: "sidebar-account",
                span { "{username}" }
                Link { to: Route::Profile {}, class: "profile-btn", "⚙" }
            }
            Link {
                to: Route::Home {},
                class: "sidebar-home",
                active_class: "active",
                "🏠 {i18n_text.home}"
            }
            h2 { class: "sidebar-title", "{i18n_text.categories}" }
            ul { class: "category-list",
                for category in store.read().categories.iter() {
                    CategoryTreeItem {
                        key: "{category.id}",
                        category: category.clone(),
                        expanded,
                    }
                }
            }
            NameCreateButton {
                title: "{i18n_text.new_category}",
                onsubmit: move |name: String| {
                    store.write().add_category(name);
                },
            }
            h2 { class: "sidebar-title sidebar-title--universes", "{i18n_text.universes}" }
            ul { class: "universe-nav",
                for universe in universe_store.read().universes.iter() {
                    li { key: "{universe.id}",
                        Link {
                            to: Route::UniverseView { id: universe.id },
                            class: "universe-nav-link",
                            active_class: "active",
                            "{universe.name}"
                        }
                    }
                }
            }
            NameCreateButton {
                title: "{i18n_text.new_universe}",
                onsubmit: move |name: String| {
                    let id = universe_store.write().add_universe(
                        name,
                        i18n_text.order_release.to_string(),
                        i18n_text.order_manual.to_string(),
                    );
                    navigator().push(Route::UniverseView { id });
                },
            }
        }
    }
}

/// Bouton "+" pleine largeur qui ouvre une pop-up demandant un nom, puis
/// appelle `onsubmit` avec le nom saisi au clic sur "Créer".
#[component]
fn NameCreateButton(title: String, onsubmit: EventHandler<String>) -> Element {
    let i18n_text = use_texts();
    let mut open = use_signal(|| false);
    let mut name = use_signal(String::new);
    let is_empty = name.read().trim().is_empty();
    let mut close = move || {
        open.set(false);
        name.set(String::new());
    };

    rsx! {
        button {
            r#type: "button",
            class: "sidebar-add-btn",
            title: "{title}",
            onclick: move |_| open.set(true),
            "+"
        }
        if open() {
            Modal { onclose: move |_| close(),
                h2 { "{title}" }
                form {
                    class: "new-item-modal-form",
                    onsubmit: move |event: FormEvent| {
                        event.prevent_default();
                        let new_name = name.read().trim().to_string();
                        if new_name.is_empty() {
                            return;
                        }
                        onsubmit.call(new_name);
                        close();
                    },
                    label {
                        "{i18n_text.name_label}"
                        input {
                            autofocus: true,
                            value: "{name}",
                            oninput: move |e| name.set(e.value()),
                        }
                    }
                    div { class: "modal-actions",
                        button {
                            r#type: "button",
                            onclick: move |_| close(),
                            "{i18n_text.cancel}"
                        }
                        button {
                            r#type: "submit",
                            class: "primary-btn",
                            disabled: is_empty,
                            "{i18n_text.create}"
                        }
                    }
                }
            }
        }
    }
}
