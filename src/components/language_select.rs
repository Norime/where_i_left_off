use dioxus::prelude::*;
use crate::i18n::Language;

/// Liste déroulante de langue (un `<select>` natif ne peut pas afficher
/// d'image) : un bouton avec le drapeau de la langue choisie ouvre un menu
/// drapeau + nom. `compact` n'affiche que le drapeau sur le bouton (coin de
/// l'écran de connexion). Un fond invisible derrière le menu le referme
/// quand on clique ailleurs.
#[component]
pub fn LanguageSelect(
    value: Language,
    onchange: EventHandler<Language>,
    #[props(default)] compact: bool,
) -> Element {
    let mut open = use_signal(|| false);
    let current_name = value.native_name();
    let options: Vec<(Language, &'static str, &'static str)> = Language::ALL
        .iter()
        .map(|language| (*language, language.code(), language.native_name()))
        .collect();

    rsx! {
        div { class: "language-select",
            button {
                r#type: "button",
                class: "language-select-toggle",
                title: "{current_name}",
                onclick: move |_| open.set(!open()),
                LanguageFlag { language: value }
                if !compact {
                    span { "{current_name}" }
                }
                span { class: "language-select-caret", "▾" }
            }
            if open() {
                div {
                    class: "language-select-backdrop",
                    onclick: move |_| open.set(false),
                }
                ul { class: "language-select-menu",
                    for (language , code , name) in options {
                        li { key: "{code}",
                            button {
                                r#type: "button",
                                class: if language == value { "language-select-option language-select-option--active" } else { "language-select-option" },
                                onclick: move |_| {
                                    open.set(false);
                                    onchange.call(language);
                                },
                                LanguageFlag { language }
                                span { "{name}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn LanguageFlag(language: Language) -> Element {
    let svg = language.flag_svg();
    rsx! {
        span { class: "language-flag", dangerous_inner_html: svg }
    }
}
