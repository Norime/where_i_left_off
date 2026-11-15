use std::time::Duration;

use dioxus::prelude::*;

use crate::i18n::use_texts;

/// Type de message : change la couleur de la barre et la durée d'affichage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Success,
    Error,
}

impl ToastKind {
    fn class(self) -> &'static str {
        match self {
            ToastKind::Info => "toast toast--info",
            ToastKind::Success => "toast toast--success",
            ToastKind::Error => "toast toast--error",
        }
    }

    /// Une erreur reste plus longtemps, le temps de la lire.
    fn duration(self) -> Duration {
        match self {
            ToastKind::Info | ToastKind::Success => Duration::from_secs(5),
            ToastKind::Error => Duration::from_secs(9),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Toast {
    id: u64,
    kind: ToastKind,
    message: String,
}

/// Messages temporaires affichés en bas à droite, sur toutes les pages.
/// Fourni une fois par `use_toast_provider` (dans `App`), récupéré n'importe
/// où avec `use_toasts`, puis :
///
/// ```ignore
/// let mut toasts = use_toasts();
/// toasts.error("Impossible de télécharger l'image");
/// ```
#[derive(Clone, Copy, PartialEq)]
pub struct Toasts {
    list: Signal<Vec<Toast>>,
    next_id: Signal<u64>,
}

impl Toasts {
    pub fn info(&mut self, message: impl Into<String>) {
        self.show(ToastKind::Info, message);
    }

    pub fn success(&mut self, message: impl Into<String>) {
        self.show(ToastKind::Success, message);
    }

    pub fn error(&mut self, message: impl Into<String>) {
        self.show(ToastKind::Error, message);
    }

    /// Affiche un message, qui se ferme seul après la durée de son type (ou
    /// au clic sur ✕). Le même message déjà affiché n'est pas répété.
    pub fn show(&mut self, kind: ToastKind, message: impl Into<String>) {
        let message = message.into();
        if self.list.peek().iter().any(|t| t.kind == kind && t.message == message) {
            return;
        }
        let id = *self.next_id.peek();
        self.next_id.set(id + 1);
        self.list.write().push(Toast { id, kind, message });

        // `spawn_forever` : le minuteur appartient à l'application, pas au
        // composant qui a affiché le message (qui peut disparaître avant).
        let mut toasts = *self;
        dioxus::core::spawn_forever(async move {
            sleep(kind.duration()).await;
            toasts.dismiss(id);
        });
    }

    pub fn dismiss(&mut self, id: u64) {
        self.list.write().retain(|t| t.id != id);
    }
}

/// Attente avant la fermeture automatique. Sur le web (WASM), pas de
/// minuterie disponible ici : le message reste jusqu'au clic sur ✕.
async fn sleep(duration: Duration) {
    #[cfg(not(target_arch = "wasm32"))]
    tokio::time::sleep(duration).await;
    #[cfg(target_arch = "wasm32")]
    std::future::pending::<()>().await;
    let _ = duration;
}

/// À appeler une seule fois, dans `App`, avant d'afficher `ToastHost`.
/// Les signaux sont créés dans la portée racine pour que les messages
/// survivent aux changements de page.
pub fn use_toast_provider() -> Toasts {
    use_context_provider(|| Toasts {
        list: Signal::new_in_scope(Vec::new(), ScopeId::ROOT),
        next_id: Signal::new_in_scope(0, ScopeId::ROOT),
    })
}

pub fn use_toasts() -> Toasts {
    use_context::<Toasts>()
}

/// Pile des messages en bas à droite. À placer une fois, dans `App`.
#[component]
pub fn ToastHost() -> Element {
    let i18n_text = use_texts();
    let mut toasts = use_toasts();
    let list = toasts.list.read().clone();

    rsx! {
        div { class: "toast-stack",
            for toast in list {
                div { key: "{toast.id}", class: toast.kind.class(), role: "status",
                    span { class: "toast-message", "{toast.message}" }
                    button {
                        r#type: "button",
                        title: "{i18n_text.close}",
                        onclick: move |_| toasts.dismiss(toast.id),
                        "✕"
                    }
                }
            }
        }
    }
}
