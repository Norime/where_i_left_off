use dioxus::prelude::*;

/// Fenêtre modale générique : un fond semi-transparent (cliquer dessus
/// ferme la modale) avec une boîte centrée contenant le contenu fourni.
/// `class` s'ajoute à celle de la boîte (ex: une modale plus large).
#[component]
pub fn Modal(onclose: EventHandler<()>, #[props(default)] class: String, children: Element) -> Element {
    rsx! {
        div { class: "modal-backdrop", onclick: move |_| onclose.call(()),
            div {
                class: "modal-box {class}",
                onclick: move |evt| evt.stop_propagation(),
                {children}
            }
        }
    }
}