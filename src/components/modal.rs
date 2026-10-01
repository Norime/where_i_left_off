use dioxus::prelude::*;

/// Fenêtre modale générique : un fond semi-transparent (cliquer dessus
/// ferme la modale) avec une boîte centrée contenant le contenu fourni.
#[component]
pub fn Modal(onclose: EventHandler<()>, children: Element) -> Element {
    rsx! {
        div { class: "modal-backdrop", onclick: move |_| onclose.call(()),
            div {
                class: "modal-box",
                onclick: move |evt| evt.stop_propagation(),
                {children}
            }
        }
    }
}