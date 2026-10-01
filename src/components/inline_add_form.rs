use dioxus::prelude::*;

/// Petit contrôle réutilisable "texte + bouton +" : appelle `onsubmit` avec
/// le texte saisi (clic sur "+" ou touche Entrée), puis vide le champ.
/// Volontairement construit sans balise `<form>` pour rester utilisable
/// même imbriqué dans un formulaire existant — les formulaires HTML ne
/// peuvent pas être imbriqués les uns dans les autres.
#[component]
pub fn InlineAddForm(placeholder: String, onsubmit: EventHandler<String>) -> Element {
    let mut value = use_signal(String::new);

    rsx! {
        div { class: "inline-add-form",
            input {
                class: "inline-add-input",
                placeholder: "{placeholder}",
                value: "{value}",
                oninput: move |event| value.set(event.value()),
                onkeydown: move |event| {
                    if let Key::Enter = event.key() {
                        let name = value.read().trim().to_string();
                        if !name.is_empty() {
                            onsubmit.call(name);
                            value.set(String::new());
                        }
                    }
                },
            }
            button {
                r#type: "button",
                class: "inline-add-btn",
                onclick: move |_| {
                    let name = value.read().trim().to_string();
                    if !name.is_empty() {
                        onsubmit.call(name);
                        value.set(String::new());
                    }
                },
                "+"
            }
        }
    }
}