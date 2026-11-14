use dioxus::prelude::*;

use crate::components::{DeleteButton, Modal};
use crate::i18n::{use_texts, Texts};
use crate::models::{
    file_slug, normalize_hex, split_alpha, with_alpha, ImageFit, ImportReport, SavedTheme, Theme, ThemeChoice, ThemeColor,
    ThemeImage, ThemeImageSlot, ThemePreset, User, MAX_GLASS_BLUR, MAX_IMAGE_BYTES,
};
use crate::persistence::{self, MediaError};
use crate::state::UserStore;

/// Message affiché sous les boutons d'import / export.
#[derive(Clone, PartialEq)]
enum ThemeMessage {
    Success(String),
    Error(String),
}

/// Réglages › Apparence › Thème : thèmes prédéfinis (non modifiables ni
/// exportables), puis thèmes créés ou importés par l'utilisateur, avec
/// import / export et l'édition du thème perso choisi. Tout s'applique
/// immédiatement, le menu à gauche sert d'aperçu.
#[component]
pub fn ThemeSection(user: User) -> Element {
    let i18n_text = use_texts();
    let mut user_store = use_context::<Signal<UserStore>>();
    let mut message = use_signal(|| None::<ThemeMessage>);
    let mut exporting = use_signal(|| false);
    let user_id = user.id;
    let choice = user.theme_choice.clone();
    let current = user.effective_theme();
    let active_saved = user.active_saved_theme().cloned();
    let can_export = active_saved.is_some();
    // Nom proposé pour un nouveau thème : « <thème actuel> (copie) ».
    let new_theme_name = match (&active_saved, &choice) {
        (Some(saved), _) => (i18n_text.theme_copy_name)(&saved.name),
        (None, ThemeChoice::Preset(key)) => ThemePreset::from_key(key)
            .map(|preset| (i18n_text.theme_copy_name)(i18n_text.theme_preset_label(preset)))
            .unwrap_or_else(|| i18n_text.new_theme_default_name.to_string()),
        (None, ThemeChoice::Custom(_)) => i18n_text.new_theme_default_name.to_string(),
    };

    let import = move |event: FormEvent| {
        let Some(file) = event.files().into_iter().next() else {
            return;
        };
        spawn(async move {
            let parsed = match file.read_string().await {
                Ok(text) => Theme::import_json(&text).ok(),
                Err(_) => None,
            };
            let Some(imported) = parsed else {
                message.set(Some(ThemeMessage::Error(i18n_text.theme_import_invalid.to_string())));
                return;
            };
            // Nom : celui du fichier de thème, sinon le nom du fichier lui-même.
            let name = imported
                .name
                .or_else(|| {
                    let file_name = file.name();
                    let stem = file_name.trim_end_matches(".json").trim();
                    (!stem.is_empty()).then(|| stem.to_string())
                })
                .unwrap_or_else(|| i18n_text.imported_theme_default.to_string());
            let mut theme = imported.theme;
            let mut report = imported.report;
            // Image valide mais impossible à écrire sur le disque : comptée
            // comme ignorée, le reste du thème est quand même importé.
            let failed = persistence::save_imported_images(user_id, &mut theme, imported.images);
            report.images_applied -= failed;
            report.images_ignored.extend(std::iter::repeat_n("?".to_string(), failed));
            user_store.write().add_saved_theme(user_id, name.clone(), theme);
            message.set(Some(ThemeMessage::Success(import_summary(i18n_text, &name, &report))));
        });
    };

    rsx! {
        h1 { "{i18n_text.theme}" }
        p { class: "profile-hint", "{i18n_text.theme_hint}" }

        h2 { "{i18n_text.theme_presets}" }
        div { class: "theme-preset-list",
            for preset in ThemePreset::ALL {
                ThemeCard {
                    key: "{preset.key()}",
                    name: i18n_text.theme_preset_label(preset).to_string(),
                    theme: preset.theme(),
                    active: choice == ThemeChoice::preset(preset),
                    onselect: move |_| user_store.write().select_theme(user_id, ThemeChoice::preset(preset)),
                }
            }
        }

        div { class: "theme-section-header",
            h2 { "{i18n_text.custom_themes}" }
            div { class: "profile-password-actions",
                label { class: "theme-import-btn",
                    "{i18n_text.import_theme}"
                    // Le vrai `<input type="file">` est caché derrière ce label.
                    input {
                        r#type: "file",
                        accept: ".json,application/json",
                        onchange: import,
                    }
                }
                button {
                    r#type: "button",
                    class: "primary-btn",
                    disabled: !can_export,
                    title: if !can_export { "{i18n_text.export_custom_only}" },
                    onclick: move |_| exporting.set(true),
                    "{i18n_text.export_theme}"
                }
            }
        }
        match message() {
            Some(ThemeMessage::Success(text)) => rsx! { p { class: "profile-success", "{text}" } },
            Some(ThemeMessage::Error(text)) => rsx! { p { class: "auth-error", "{text}" } },
            None => rsx! {},
        }
        div { class: "theme-preset-list",
            for saved in user.saved_themes.iter().cloned() {
                ThemeCard {
                    key: "{saved.id}",
                    name: saved.name.clone(),
                    theme: saved.theme.clone(),
                    active: choice == ThemeChoice::Custom(saved.id),
                    onselect: move |_| user_store.write().select_theme(user_id, ThemeChoice::Custom(saved.id)),
                }
            }
            // Crée tout de suite le thème (copie du thème actuel) et le
            // choisit : son nom et ses couleurs s'éditent juste en dessous.
            button {
                r#type: "button",
                class: "theme-preset theme-preset--new",
                onclick: move |_| {
                    user_store.write().add_saved_theme(user_id, new_theme_name.clone(), current.clone());
                    message.set(None);
                },
                div { class: "theme-preview theme-preview--new", "+" }
                span { "{i18n_text.create_theme}" }
            }
        }
        if user.saved_themes.is_empty() {
            p { class: "profile-hint", "{i18n_text.no_custom_themes}" }
        }

        if let (true, Some(saved)) = (exporting(), active_saved.clone()) {
            ExportThemeModal { user_id, saved, message, onclose: move |_| exporting.set(false) }
        }

        match active_saved {
            Some(saved) => rsx! {
                SavedThemeEditor { key: "{saved.id}", user_id, saved }
            },
            None => rsx! {
                p { class: "profile-hint", "{i18n_text.preset_not_editable}" }
            },
        }
    }
}

/// Carte cliquable d'un thème : mini aperçu + nom.
#[component]
fn ThemeCard(name: String, theme: Theme, active: bool, onselect: EventHandler<()>) -> Element {
    rsx! {
        button {
            r#type: "button",
            class: if active { "theme-preset theme-preset--active" } else { "theme-preset" },
            title: "{name}",
            onclick: move |_| onselect.call(()),
            ThemePreview { theme }
            span { class: "theme-preset-name", "{name}" }
        }
    }
}

/// Miniature d'un thème : le menu à gauche, une page avec une carte, un
/// bouton (couleur principale) et une pastille (accent).
#[component]
fn ThemePreview(theme: Theme) -> Element {
    let menu = theme.get(ThemeColor::MenuBackground);
    let highlight = theme.get(ThemeColor::MenuHighlight);
    let page = theme.get(ThemeColor::PageBackground);
    let primary = theme.get(ThemeColor::Primary);
    let accent = theme.get(ThemeColor::Accent);
    let surface = theme.get(ThemeColor::Surface);
    rsx! {
        div { class: "theme-preview", style: "background: {page};",
            div { class: "theme-preview-menu", style: "background: {menu};",
                div { class: "theme-preview-highlight", style: "background: {highlight};" }
            }
            div { class: "theme-preview-page",
                div { class: "theme-preview-card", style: "background: {surface};" }
                div { class: "theme-preview-row",
                    div { class: "theme-preview-button", style: "background: {primary};" }
                    div { class: "theme-preview-dot", style: "background: {accent};" }
                }
            }
        }
    }
}

/// Export du thème perso choisi, sous son nom, avec ou sans ses images.
#[component]
fn ExportThemeModal(user_id: usize, saved: SavedTheme, message: Signal<Option<ThemeMessage>>, onclose: EventHandler<()>) -> Element {
    let i18n_text = use_texts();
    let has_images = !saved.theme.images().is_empty();
    let mut include_images = use_signal(|| has_images);
    let images_size = (i18n_text.file_size)(persistence::media_size(user_id, &saved.theme));

    let title = (i18n_text.export_title)(&saved.name);

    let export = move |_| {
        let images = if include_images() {
            persistence::load_theme_images(user_id, &saved.theme)
        } else {
            Default::default()
        };
        let result = persistence::export_theme(&file_slug(&saved.name), &saved.theme.export_json(&saved.name, &images));
        message.set(Some(match result {
            Ok(path) => ThemeMessage::Success((i18n_text.theme_exported)(&path.display().to_string())),
            Err(error) => ThemeMessage::Error((i18n_text.theme_export_failed)(&error.to_string())),
        }));
        onclose.call(());
    };

    rsx! {
        Modal { onclose: move |_| onclose.call(()),
            h2 { "{title}" }
            if has_images {
                label { class: "theme-export-option",
                    input {
                        r#type: "checkbox",
                        checked: include_images(),
                        onchange: move |e| include_images.set(e.checked()),
                    }
                    {(i18n_text.export_include_images)(&images_size)}
                }
                p { class: "profile-hint", "{i18n_text.export_images_hint}" }
            } else {
                p { class: "profile-hint", "{i18n_text.export_no_images}" }
            }
            div { class: "modal-actions",
                button { r#type: "button", onclick: move |_| onclose.call(()), "{i18n_text.cancel}" }
                button { r#type: "button", class: "primary-btn", onclick: export, "{i18n_text.export_theme}" }
            }
        }
    }
}

/// Modification du thème perso choisi : nom, couleurs, images, effet verre,
/// suppression.
#[component]
fn SavedThemeEditor(user_id: usize, saved: SavedTheme) -> Element {
    let i18n_text = use_texts();
    let mut user_store = use_context::<Signal<UserStore>>();
    let theme_id = saved.id;
    let mut name = use_signal(|| saved.name.clone());
    let glass_blur = saved.theme.glass_blur();
    // Le flou ne se voit qu'à travers une transparence posée sur une image :
    // on prévient plutôt que de laisser croire que le réglage ne fait rien.
    let see_through = [ThemeColor::MenuBackground, ThemeColor::Surface]
        .iter()
        .any(|color| split_alpha(&saved.theme.get(*color)).1 < 100);
    let glass_warning = if saved.theme.image(ThemeImageSlot::PageBackground).is_none() {
        Some(i18n_text.glass_needs_image)
    } else if !see_through {
        Some(i18n_text.glass_needs_transparency)
    } else {
        None
    };

    rsx! {
        div { class: "theme-editor-header",
            h2 { {(i18n_text.edit_theme)(&saved.name)} }
            div { class: "theme-editor-delete", title: "{i18n_text.delete_theme}",
                DeleteButton { onconfirm: move |_| user_store.write().delete_saved_theme(user_id, theme_id) }
            }
        }
        label { class: "theme-name-field",
            "{i18n_text.theme_name}"
            input {
                value: "{name}",
                oninput: move |e| {
                    name.set(e.value());
                    user_store.write().rename_saved_theme(user_id, theme_id, e.value());
                },
                // Nom vidé puis abandonné : on réaffiche le nom enregistré.
                onblur: move |_| {
                    if name.read().trim().is_empty() {
                        name.set(saved.name.clone());
                    }
                },
            }
        }

        h3 { "{i18n_text.colors_title}" }
        p { class: "profile-hint", "{i18n_text.colors_hint}" }
        div { class: "theme-color-list",
            for color in ThemeColor::ALL {
                ThemeColorRow {
                    key: "{color.key()}",
                    user_id,
                    theme_id,
                    color,
                    value: saved.theme.get(color).to_string(),
                }
            }
        }

        h3 { "{i18n_text.images_title}" }
        p { class: "profile-hint", "{i18n_text.images_hint}" }
        div { class: "theme-image-list",
            for slot in ThemeImageSlot::ALL {
                ThemeImageRow {
                    key: "{slot.key()}",
                    user_id,
                    theme_id,
                    slot,
                    image: saved.theme.image(slot).cloned(),
                }
            }
        }

        h3 { "{i18n_text.glass_title}" }
        p { class: "profile-hint", "{i18n_text.glass_hint}" }
        SliderInput {
            value: glass_blur,
            max: MAX_GLASS_BLUR,
            unit: "px",
            onchange: move |blur: u8| {
                user_store.write().edit_saved_theme(user_id, theme_id, |theme| theme.set_glass_blur(blur));
            },
        }
        if let Some(warning) = glass_warning {
            p { class: "theme-warning", "{warning}" }
        }
    }
}

/// Une couleur d'un thème perso : sélecteur de couleur, opacité et code
/// (`#rrggbb` ou `#rrggbbaa`). Le texte tapé est gardé dans `draft` tant
/// qu'il n'est pas un code complet, pour pouvoir l'écrire caractère par
/// caractère.
#[component]
fn ThemeColorRow(user_id: usize, theme_id: usize, color: ThemeColor, value: String) -> Element {
    let i18n_text = use_texts();
    let mut user_store = use_context::<Signal<UserStore>>();
    let mut draft = use_signal(|| None::<String>);
    let shown = draft().unwrap_or_else(|| value.clone());
    let (rgb, opacity) = split_alpha(&value);
    let rgb_for_slider = rgb.clone();

    rsx! {
        div { class: "theme-color-row",
            span { class: "theme-color-label", "{i18n_text.theme_color_label(color)}" }
            // Le sélecteur natif ne gère pas la transparence : il change
            // seulement la teinte, l'opacité actuelle est gardée.
            input {
                r#type: "color",
                value: "{rgb}",
                oninput: move |event| {
                    draft.set(None);
                    let value = with_alpha(&event.value(), opacity);
                    user_store.write().set_saved_theme_color(user_id, theme_id, color, &value);
                },
            }
            span { title: "{i18n_text.opacity}",
                SliderInput {
                    value: opacity,
                    max: 100,
                    unit: "%",
                    onchange: move |opacity: u8| {
                        draft.set(None);
                        let value = with_alpha(&rgb_for_slider, opacity);
                        user_store.write().set_saved_theme_color(user_id, theme_id, color, &value);
                    },
                }
            }
            input {
                class: "theme-color-code",
                value: "{shown}",
                maxlength: 9,
                spellcheck: false,
                oninput: move |event| {
                    let text = event.value();
                    // Seul un code complet est appliqué : `#abc` serait
                    // accepté trop tôt, en pleine saisie.
                    let complete = matches!(text.trim().len(), 7 | 9);
                    if complete && normalize_hex(&text).is_some() {
                        user_store.write().set_saved_theme_color(user_id, theme_id, color, &text);
                        draft.set(None);
                    } else {
                        draft.set(Some(text));
                    }
                },
                onblur: move |_| draft.set(None),
            }
        }
    }
}

/// Image d'une zone : aperçu, choix du fichier (copié dans le dossier du
/// compte), affichage, voile et retrait.
#[component]
fn ThemeImageRow(user_id: usize, theme_id: usize, slot: ThemeImageSlot, image: Option<ThemeImage>) -> Element {
    let i18n_text = use_texts();
    let mut user_store = use_context::<Signal<UserStore>>();
    let mut error = use_signal(|| None::<String>);
    // L'éditeur ne s'affiche que pour le thème choisi : son image est déjà
    // chargée dans la variable CSS de la zone.
    let preview_style = format!("background-image: var({});", slot.css_variable());

    let choose = {
        let image = image.clone();
        move |event: FormEvent| {
            let Some(file) = event.files().into_iter().next() else { return };
            let previous = image.clone();
            if file.size() > MAX_IMAGE_BYTES as u64 {
                error.set(Some((i18n_text.image_too_large)(&(i18n_text.file_size)(MAX_IMAGE_BYTES as u64))));
                return;
            }
            spawn(async move {
                let saved = match file.read_bytes().await {
                    Ok(bytes) => persistence::save_media(user_id, &bytes),
                    Err(e) => Err(MediaError::Io(e.to_string())),
                };
                match saved {
                    Ok(file) => {
                        // Remplacement : on garde l'affichage et le voile choisis.
                        let new_image = match previous {
                            Some(previous) => ThemeImage { file, ..previous },
                            None => ThemeImage::new(file),
                        };
                        user_store
                            .write()
                            .edit_saved_theme(user_id, theme_id, |theme| theme.set_image(slot, Some(new_image)));
                        error.set(None);
                    }
                    Err(MediaError::Unsupported) => error.set(Some(i18n_text.image_unsupported.to_string())),
                    Err(MediaError::TooLarge) => {
                        error.set(Some((i18n_text.image_too_large)(&(i18n_text.file_size)(MAX_IMAGE_BYTES as u64))))
                    }
                    Err(MediaError::Io(detail)) => error.set(Some((i18n_text.image_save_failed)(&detail))),
                }
            });
        }
    };

    rsx! {
        div { class: "theme-image-row",
            div {
                class: if image.is_some() { "theme-image-preview" } else { "theme-image-preview theme-image-preview--empty" },
                style: if image.is_some() { "{preview_style}" },
            }
            div { class: "theme-image-body",
                span { class: "theme-color-label", "{i18n_text.image_slot_label(slot)}" }
                div { class: "theme-image-actions",
                    label { class: "theme-import-btn",
                        if image.is_some() { "{i18n_text.change_image}" } else { "{i18n_text.choose_image}" }
                        input {
                            r#type: "file",
                            accept: "image/png,image/jpeg,image/gif,image/webp,image/bmp,image/avif",
                            onchange: choose,
                        }
                    }
                    if image.is_some() {
                        button {
                            r#type: "button",
                            class: "theme-import-btn",
                            onclick: move |_| {
                                user_store.write().edit_saved_theme(user_id, theme_id, |theme| theme.set_image(slot, None));
                            },
                            "{i18n_text.remove_image}"
                        }
                    }
                }
                if let Some(current) = image.clone() {
                    div { class: "theme-image-options",
                        label {
                            "{i18n_text.image_fit}"
                            select {
                                value: "{current.fit.key()}",
                                onchange: move |e| {
                                    let fit = ImageFit::from_key(&e.value()).unwrap_or_default();
                                    user_store.write().edit_saved_theme(user_id, theme_id, |theme| {
                                        if let Some(image) = theme.image(slot).cloned() {
                                            theme.set_image(slot, Some(ThemeImage { fit, ..image }));
                                        }
                                    });
                                },
                                for fit in ImageFit::ALL {
                                    option { key: "{fit.key()}", value: "{fit.key()}", "{i18n_text.image_fit_label(fit)}" }
                                }
                            }
                        }
                        div { class: "theme-slider-row",
                            "{i18n_text.image_opacity}"
                            SliderInput {
                                value: current.opacity,
                                max: 100,
                                unit: "%",
                                onchange: move |opacity: u8| {
                                    user_store.write().edit_saved_theme(user_id, theme_id, |theme| {
                                        if let Some(image) = theme.image(slot).cloned() {
                                            theme.set_image(slot, Some(ThemeImage { opacity, ..image }));
                                        }
                                    });
                                },
                            }
                        }
                    }
                }
                if let Some(message) = error() {
                    p { class: "auth-error", "{message}" }
                }
            }
        }
    }
}

/// Curseur et champ numérique liés : le curseur pour régler vite, le champ
/// pour taper une valeur précise. Une saisie incomplète ou hors limites
/// (`draft`) n'est appliquée qu'une fois valide.
#[component]
fn SliderInput(value: u8, max: u8, unit: &'static str, onchange: EventHandler<u8>) -> Element {
    let mut draft = use_signal(|| None::<String>);
    let shown = draft().unwrap_or_else(|| value.to_string());

    rsx! {
        span { class: "theme-slider",
            input {
                r#type: "range",
                min: 0,
                max: max as i64,
                value: value as i64,
                oninput: move |e| {
                    draft.set(None);
                    if let Ok(v) = e.value().parse::<u8>() {
                        onchange.call(v.min(max));
                    }
                },
            }
            input {
                r#type: "number",
                class: "theme-number",
                min: 0,
                max: max as i64,
                value: "{shown}",
                oninput: move |e| {
                    let text = e.value();
                    match text.trim().parse::<u16>() {
                        Ok(v) if v <= max as u16 => {
                            onchange.call(v as u8);
                            draft.set(None);
                        }
                        _ => draft.set(Some(text)),
                    }
                },
                onblur: move |_| draft.set(None),
            }
            span { class: "theme-slider-unit", "{unit}" }
        }
    }
}

fn import_summary(i18n_text: &Texts, name: &str, report: &ImportReport) -> String {
    let mut summary = (i18n_text.theme_imported)(name, report.applied, report.applied + report.missing);
    if !report.ignored.is_empty() {
        summary.push(' ');
        summary.push_str(&(i18n_text.theme_import_ignored)(&report.ignored.join(", ")));
    }
    if report.images_applied > 0 {
        summary.push(' ');
        summary.push_str(&(i18n_text.theme_import_images)(report.images_applied));
    }
    if !report.images_ignored.is_empty() {
        summary.push(' ');
        summary.push_str(&(i18n_text.theme_import_images_ignored)(report.images_ignored.len()));
    }
    summary
}
