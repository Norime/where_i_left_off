use dioxus::prelude::*;

use crate::components::use_toasts;
use crate::i18n::use_texts;
use crate::models::MAX_IMAGE_BYTES;
use crate::net::{self, ImageDownloadError};

/// Bouton « Depuis une URL » : affiche un champ d'adresse, télécharge
/// l'image une seule fois et passe son contenu à `onloaded`, qui la copie
/// dans les données de l'application comme un fichier choisi sur le disque.
/// Un échec de téléchargement est signalé par un toast ; une adresse mal
/// formée, sous le champ.
#[component]
pub fn UrlImageButton(onloaded: EventHandler<Vec<u8>>) -> Element {
    let i18n_text = use_texts();
    let mut toasts = use_toasts();
    let mut open = use_signal(|| false);
    let mut url = use_signal(String::new);
    let mut downloading = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    let mut download = move |_: ()| {
        if downloading() {
            return;
        }
        let address = url();
        downloading.set(true);
        error.set(None);
        spawn(async move {
            let result = net::fetch_image(&address, MAX_IMAGE_BYTES).await;
            downloading.set(false);
            match result {
                Ok(bytes) => {
                    onloaded.call(bytes);
                    open.set(false);
                    url.set(String::new());
                }
                Err(ImageDownloadError::InvalidUrl) => error.set(Some(i18n_text.image_url_invalid.to_string())),
                Err(ImageDownloadError::Unreachable(detail)) => toasts.error((i18n_text.image_download_failed)(&detail)),
                Err(ImageDownloadError::HttpStatus(code)) => toasts.error((i18n_text.image_download_http)(code)),
                Err(ImageDownloadError::TooLarge) => {
                    toasts.error((i18n_text.image_too_large)(&(i18n_text.file_size)(MAX_IMAGE_BYTES as u64)))
                }
            }
        });
    };

    if !open() {
        return rsx! {
            button { r#type: "button", class: "theme-import-btn", onclick: move |_| open.set(true), "🔗 {i18n_text.from_url}" }
        };
    }

    rsx! {
        div { class: "url-image-form",
            input {
                r#type: "url",
                placeholder: "https://…",
                value: "{url}",
                disabled: downloading(),
                oninput: move |e| url.set(e.value()),
                onkeydown: move |e| {
                    if e.key() == Key::Enter {
                        e.prevent_default();
                        download(());
                    }
                },
            }
            button {
                r#type: "button",
                class: "primary-btn",
                disabled: downloading() || url.read().trim().is_empty(),
                onclick: move |_| download(()),
                if downloading() { "{i18n_text.downloading}" } else { "{i18n_text.download}" }
            }
            button {
                r#type: "button",
                class: "theme-import-btn",
                disabled: downloading(),
                onclick: move |_| {
                    open.set(false);
                    error.set(None);
                },
                "{i18n_text.cancel}"
            }
            if let Some(message) = error() {
                p { class: "auth-error url-image-error", "{message}" }
            }
        }
    }
}
