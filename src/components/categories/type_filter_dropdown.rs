use dioxus::prelude::*;
use std::collections::HashSet;

use crate::components::TypeCheckboxList;

/// Enveloppe `TypeCheckboxList` derrière un bouton qui ouvre/ferme un panneau
/// déroulant (au lieu d'afficher les cases à cocher directement dans la
/// barre d'outils). Même signature que `TypeCheckboxList` pour pouvoir le
/// remplacer directement là où il servait de filtre.
#[component]
pub fn TypeFilterDropdown(category_ids: Vec<usize>, selected: Signal<HashSet<usize>>) -> Element {
    let mut show_panel = use_signal(|| false);
    let selected_count = selected.read().len();
    let button_label = if selected_count == 0 {
        "Type : Tous".to_string()
    } else {
        format!("Type : {selected_count} sélectionné(s)")
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