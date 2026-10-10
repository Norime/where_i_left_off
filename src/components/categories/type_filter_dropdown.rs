use dioxus::prelude::*;
use std::collections::HashSet;

use crate::components::TypeCheckboxList;
use crate::i18n::use_texts;

/// Enveloppe `TypeCheckboxList` derrière un bouton qui ouvre/ferme un panneau
/// déroulant (au lieu d'afficher les cases à cocher directement dans la
/// barre d'outils). Même signature que `TypeCheckboxList` pour pouvoir le
/// remplacer directement là où il servait de filtre.
#[component]
pub fn TypeFilterDropdown(category_ids: Vec<usize>, selected: Signal<HashSet<usize>>) -> Element {
    let i18n_text = use_texts();
    let mut show_panel = use_signal(|| false);
    let selected_count = selected.read().len();
    let button_label = if selected_count == 0 {
        i18n_text.type_filter_all.to_string()
    } else {
        (i18n_text.type_filter_selected)(selected_count)
    };

    rsx! {
        div { class: "type-filter-dropdown",
            button {
                r#type: "button",
                class: "toolbar-control type-filter-toggle",
                onclick: move |_| show_panel.set(!show_panel()),
                "{button_label}"
                span { class: "type-filter-caret", "▾" }
            }
            if show_panel() {
                div { class: "type-filter-panel",
                    TypeCheckboxList { category_ids: category_ids.clone(), selected }
                }
            }
        }
    }
}