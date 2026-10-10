use dioxus::prelude::*;
use std::collections::HashSet;
use crate::components::{CategoryTreeItem, InlineAddForm};
use crate::i18n::use_texts;
use crate::routes::Route;
use crate::state::{CategoryStore, UserStore};

/// Menu de gauche : compte connecté avec accès aux réglages du profil
/// (roue crantée), arborescence des catégories/sous-catégories, et
/// formulaire pour ajouter une nouvelle catégorie.
#[component]
pub fn Sidebar() -> Element {
    let i18n_text = use_texts();
    let mut store = use_context::<Signal<CategoryStore>>();
    let session = use_context::<Signal<Option<usize>>>();
    let user_store = use_context::<Signal<UserStore>>();
    let expanded = use_signal(HashSet::<usize>::new);

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
            InlineAddForm {
                placeholder: "{i18n_text.new_category}",
                onsubmit: move |name: String| {
                    store.write().add_category(name);
                },
            }
        }
    }
}