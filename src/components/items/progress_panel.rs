use dioxus::prelude::*;
use std::collections::HashSet;

use crate::components::{DeleteButton, Modal};
use crate::i18n::{use_texts, Language};
use crate::models::{ItemInfoField, ProgressLink, ProgressNode, ProgressNodeKind};
use crate::state::ItemStore;

/// Panneau de suivi de progression d'un placement précis (une catégorie +
/// sous-catégorie données).
///
/// Structure imposée : la racine est toujours une liste ("Group", juste un
/// titre). Chaque liste (racine ou imbriquée) déclare, DÈS SA CRÉATION, le
/// type de ses enfants — sous-listes, texte, ou cartes — via son champ
/// `children_kind` : impossible de mélanger plusieurs types dans une même
/// liste. Le statut de l'item (Nouveau/En cours/Terminé) est recalculé
/// automatiquement par le store à chaque changement de progression.
#[component]
pub fn ProgressPanel(item_id: usize, category_id: usize, subcategory_id: usize) -> Element {
    let item_store = use_context::<Signal<ItemStore>>();
    let expanded = use_signal(HashSet::<usize>::new);
    let editing_card = use_signal(|| None::<usize>);

    let progress: Vec<ProgressNode> = item_store
        .read()
        .items
        .iter()
        .find(|i| i.id == item_id)
        .and_then(|i| i.placements.iter().find(|p| p.category_id == category_id && p.subcategory_id == subcategory_id))
        .map(|p| p.progress.clone())
        .unwrap_or_default();

    rsx! {
        div { class: "progress-panel",
            ProgressNodeList {
                item_id,
                category_id,
                subcategory_id,
                all_nodes: progress,
                parent_id: None,
                children_kind: Some(ProgressNodeKind::Group),
                expanded,
                editing_card,
            }
        }
    }
}

/// Cherche les liens effectifs d'un nœud : les siens s'il en a, sinon ceux
/// du premier ancêtre qui en a (en remontant `parent_id`), sinon rien.
/// Renvoie aussi si le résultat vient d'un ancêtre (hérité) ou du nœud
/// lui-même.
fn effective_links_for(all_nodes: &[ProgressNode], node_id: usize) -> (Vec<ProgressLink>, bool) {
    let mut current_id = Some(node_id);
    while let Some(id) = current_id {
        if let Some(node) = all_nodes.iter().find(|n| n.id == id) {
            if !node.redirect_links.is_empty() {
                return (node.redirect_links.clone(), id != node_id);
            }
            current_id = node.parent_id;
        } else {
            break;
        }
    }
    (Vec::new(), false)
}

/// Vrai si TOUS les descendants "feuilles" (Text/Card, à n'importe quelle
/// profondeur) d'un nœud sont cochés. Faux si le nœud n'a aucune feuille
/// descendante (pas de "tout cocher" vide qui s'affiche coché).
fn all_descendants_done(all_nodes: &[ProgressNode], node_id: usize) -> bool {
    let mut to_check = vec![node_id];
    let mut leaves: Vec<&ProgressNode> = Vec::new();
    let mut index = 0;
    while index < to_check.len() {
        let current = to_check[index];
        for node in all_nodes.iter() {
            if node.parent_id == Some(current) {
                to_check.push(node.id);
                if node.kind != ProgressNodeKind::Group {
                    leaves.push(node);
                }
            }
        }
        index += 1;
    }
    !leaves.is_empty() && leaves.iter().all(|n| n.is_done)
}

/// Affiche tous les nœuds directement enfants de `parent_id` (les racines
/// si `parent_id` vaut `None`), sous la forme adaptée à `children_kind` —
/// galerie de cartes, liste compacte de texte, ou liste de sous-listes —
/// et ajoute au bout le bon moyen d'en créer un nouveau.
#[component]
fn ProgressNodeList(
    item_id: usize,
    category_id: usize,
    subcategory_id: usize,
    all_nodes: Vec<ProgressNode>,
    parent_id: Option<usize>,
    children_kind: Option<ProgressNodeKind>,
    expanded: Signal<HashSet<usize>>,
    editing_card: Signal<Option<usize>>,
) -> Element {
    let nodes: Vec<ProgressNode> = all_nodes.iter().filter(|n| n.parent_id == parent_id).cloned().collect();

    match children_kind {
        Some(ProgressNodeKind::Card) => rsx! {
            div { class: "progress-card-row",
                for node in nodes {
                    ProgressNodeRow {
                        key: "{node.id}",
                        item_id,
                        category_id,
                        subcategory_id,
                        all_nodes: all_nodes.clone(),
                        node,
                        expanded,
                        editing_card,
                    }
                }
                CardAddTile {
                    item_id,
                    category_id,
                    subcategory_id,
                    parent_id,
                    editing_card,
                }
            }
        },
        Some(ProgressNodeKind::Text) => rsx! {
            div { class: "progress-text-list",
                for node in nodes {
                    ProgressNodeRow {
                        key: "{node.id}",
                        item_id,
                        category_id,
                        subcategory_id,
                        all_nodes: all_nodes.clone(),
                        node,
                        expanded,
                        editing_card,
                    }
                }
                TextCreateForm {
                    item_id,
                    category_id,
                    subcategory_id,
                    parent_id,
                }
            }
        },
        // `Group`, ou nœud créé avant l'ajout de `children_kind` : on
        // retombe sur une liste de sous-listes par défaut.
        _ => rsx! {
            div { class: "progress-node-list",
                for node in nodes {
                    ProgressNodeRow {
                        key: "{node.id}",
                        item_id,
                        category_id,
                        subcategory_id,
                        all_nodes: all_nodes.clone(),
                        node,
                        expanded,
                        editing_card,
                    }
                }
                GroupCreateForm {
                    item_id,
                    category_id,
                    subcategory_id,
                    parent_id,
                }
            }
        },
    }
}

/// Une seule ligne/carte de progression, selon `node.kind` :
/// - `Group` : titre + triangle pour déplier ses enfants, plus une case à
///   cocher "en cascade" qui reflète/impose l'état de TOUS ses
///   descendants (feuilles à n'importe quelle profondeur).
/// - `Text` : ligne compacte cochable (ex: un chapitre), avec un bouton
///   "œil" pour ouvrir les liens de redirection.
/// - `Card` : vignette façon Crunchyroll (image, durée en bas à droite) ;
///   la roue crantée ouvre la config, un clic ailleurs sur la carte ouvre
///   les liens de redirection.
#[component]
fn ProgressNodeRow(
    item_id: usize,
    category_id: usize,
    subcategory_id: usize,
    all_nodes: Vec<ProgressNode>,
    node: ProgressNode,
    expanded: Signal<HashSet<usize>>,
    editing_card: Signal<Option<usize>>,
) -> Element {
    let i18n_text = use_texts();
    let mut item_store = use_context::<Signal<ItemStore>>();
    let mut show_links = use_signal(|| false);
    let node_id = node.id;
    let kind = node.kind;
    let is_expanded = expanded.read().contains(&node_id);
    let is_editing = editing_card() == Some(node_id);
    // Le label est une donnée saisie par l'utilisateur : on reconnaît le mot
    // "durée" dans toutes les langues, pas seulement la langue active.
    let duration = node
        .fields
        .iter()
        .find(|f| Language::ALL.iter().any(|l| l.texts().duration_label == f.label))
        .map(|f| f.value.clone());

    match kind {
        ProgressNodeKind::Card => {
            let (links, links_inherited) = effective_links_for(&all_nodes, node_id);
            rsx! {
                div { class: "progress-card-tile",
                    div {
                        class: "progress-card-thumb",
                        title: "{i18n_text.watch}",
                        style: node.image_url
                            .as_ref()
                            .map(|url| format!("background-image: url('{url}');"))
                            .unwrap_or_default(),
                        onclick: move |_| show_links.set(true),
                        input {
                            r#type: "checkbox",
                            class: "progress-card-checkbox",
                            checked: node.is_done,
                            onclick: move |event| event.stop_propagation(),
                            oninput: move |event| {
                                let checked = event.value().parse().unwrap_or(false);
                                item_store
                                    .write()
                                    .set_progress_node_done(
                                        item_id,
                                        category_id,
                                        subcategory_id,
                                        node_id,
                                        checked,
                                    );
                            },
                        }
                        button {
                            r#type: "button",
                            class: "progress-card-gear",
                            onclick: move |event| {
                                event.stop_propagation();
                                editing_card.set(Some(node_id));
                            },
                            "⚙"
                        }
                        if let Some(duration) = duration {
                            span { class: "progress-card-duration", "{duration}" }
                        }
                    }
                    span { class: "progress-card-title", "{node.title}" }

                    if is_editing {
                        ProgressCardConfigModal {
                            item_id,
                            category_id,
                            subcategory_id,
                            node_id,
                            title: node.title.clone(),
                            image_url: node.image_url.clone(),
                            fields: node.fields.clone(),
                            onclose: move |_| editing_card.set(None),
                        }
                    }
                    if show_links() {
                        ProgressLinksPopup {
                            item_id,
                            category_id,
                            subcategory_id,
                            node_id,
                            links,
                            is_inherited: links_inherited,
                            onclose: move |_| show_links.set(false),
                        }
                    }
                }
            }
        }
        ProgressNodeKind::Group => {
            let all_done = all_descendants_done(&all_nodes, node_id);
            rsx! {
                div { class: "progress-node progress-group",
                    div { class: "progress-node-row",
                        button {
                            r#type: "button",
                            class: "progress-toggle",
                            onclick: move |_| {
                                let mut set = expanded.write();
                                if set.contains(&node_id) {
                                    set.remove(&node_id);
                                } else {
                                    set.insert(node_id);
                                }
                            },
                            if is_expanded {
                                "▼"
                            } else {
                                "▶"
                            }
                        }
                        input {
                            r#type: "checkbox",
                            class: "progress-group-checkbox",
                            title: "{i18n_text.check_all}",
                            checked: all_done,
                            oninput: move |event| {
                                let checked = event.value().parse().unwrap_or(false);
                                item_store
                                    .write()
                                    .set_progress_subtree_done(
                                        item_id,
                                        category_id,
                                        subcategory_id,
                                        node_id,
                                        checked,
                                    );
                            },
                        }
                        input {
                            r#type: "text",
                            class: "progress-node-title",
                            value: "{node.title}",
                            oninput: move |event| {
                                item_store
                                    .write()
                                    .set_progress_node_title(
                                        item_id,
                                        category_id,
                                        subcategory_id,
                                        node_id,
                                        event.value(),
                                    );
                            },
                        }
                        DeleteButton {
                            onconfirm: move |_| {
                                item_store
                                    .write()
                                    .remove_progress_node(item_id, category_id, subcategory_id, node_id);
                            },
                        }
                    }

                    if is_expanded {
                        div { class: "progress-children",
                            ProgressNodeList {
                                item_id,
                                category_id,
                                subcategory_id,
                                all_nodes: all_nodes.clone(),
                                parent_id: Some(node_id),
                                children_kind: node.children_kind,
                                expanded,
                                editing_card,
                            }
                        }
                    }
                }
            }
        }
        ProgressNodeKind::Text => {
            let (links, links_inherited) = effective_links_for(&all_nodes, node_id);
            rsx! {
                div { class: "progress-text-row",
                    input {
                        r#type: "checkbox",
                        checked: node.is_done,
                        oninput: move |event| {
                            let checked = event.value().parse().unwrap_or(false);
                            item_store
                                .write()
                                .set_progress_node_done(
                                    item_id,
                                    category_id,
                                    subcategory_id,
                                    node_id,
                                    checked,
                                );
                        },
                    }
                    input {
                        r#type: "text",
                        class: "progress-text-title",
                        value: "{node.title}",
                        oninput: move |event| {
                            item_store
                                .write()
                                .set_progress_node_title(
                                    item_id,
                                    category_id,
                                    subcategory_id,
                                    node_id,
                                    event.value(),
                                );
                        },
                    }
                    button {
                        r#type: "button",
                        class: "progress-text-watch",
                        title: "{i18n_text.watch}",
                        onclick: move |_| show_links.set(true),
                        "👁"
                    }
                    DeleteButton {
                        onconfirm: move |_| {
                            item_store
                                .write()
                                .remove_progress_node(item_id, category_id, subcategory_id, node_id);
                        },
                    }
                    ProgressFieldsEditor {
                        item_id,
                        category_id,
                        subcategory_id,
                        node_id,
                        kind,
                        fields: node.fields.clone(),
                        image_url: node.image_url.clone(),
                    }
                    if show_links() {
                        ProgressLinksPopup {
                            item_id,
                            category_id,
                            subcategory_id,
                            node_id,
                            links,
                            is_inherited: links_inherited,
                            onclose: move |_| show_links.set(false),
                        }
                    }
                }
            }
        }
    }
}

/// Popup de configuration d'une carte (titre, image, champs libres,
/// suppression) — remplace l'ancien affichage "en dessous de la carte".
#[component]
fn ProgressCardConfigModal(
    item_id: usize,
    category_id: usize,
    subcategory_id: usize,
    node_id: usize,
    title: String,
    image_url: Option<String>,
    fields: Vec<ItemInfoField>,
    onclose: EventHandler<()>,
) -> Element {
    let i18n_text = use_texts();
    let mut item_store = use_context::<Signal<ItemStore>>();

    rsx! {
        Modal { onclose: move |_| onclose.call(()),
            div { class: "progress-card-modal",
                h3 { "{i18n_text.edit_card}" }
                div { class: "item-form-field",
                    label { "{i18n_text.title}" }
                    input {
                        r#type: "text",
                        value: "{title}",
                        oninput: move |event| {
                            item_store
                                .write()
                                .set_progress_node_title(
                                    item_id,
                                    category_id,
                                    subcategory_id,
                                    node_id,
                                    event.value(),
                                );
                        },
                    }
                }
                ProgressFieldsEditor {
                    item_id,
                    category_id,
                    subcategory_id,
                    node_id,
                    kind: ProgressNodeKind::Card,
                    fields: fields.clone(),
                    image_url: image_url.clone(),
                }
                div { class: "modal-actions",
                    DeleteButton {
                        onconfirm: move |_| {
                            item_store
                                .write()
                                .remove_progress_node(item_id, category_id, subcategory_id, node_id);
                            onclose.call(());
                        },
                    }
                    button {
                        r#type: "button",
                        class: "primary-btn",
                        onclick: move |_| onclose.call(()),
                        "{i18n_text.close}"
                    }
                }
            }
        }
    }
}

/// Popup "Regarder" : liste les liens de redirection effectifs (propres au
/// nœud, ou hérités d'un ancêtre s'il n'en a pas), avec un mini-formulaire
/// pour en ajouter (ce qui crée toujours des liens PROPRES à ce nœud,
/// et prend donc le pas sur l'héritage à partir de ce moment).
#[component]
fn ProgressLinksPopup(
    item_id: usize,
    category_id: usize,
    subcategory_id: usize,
    node_id: usize,
    links: Vec<ProgressLink>,
    is_inherited: bool,
    onclose: EventHandler<()>,
) -> Element {
    let i18n_text = use_texts();
    let mut item_store = use_context::<Signal<ItemStore>>();
    let mut new_label = use_signal(String::new);
    let mut new_url = use_signal(String::new);

    rsx! {
        Modal { onclose: move |_| onclose.call(()),
            div { class: "progress-links-modal",
                h3 { "{i18n_text.watch}" }
                if is_inherited {
                    p { class: "toolbar-hint", "{i18n_text.inherited_links}" }
                }
                if links.is_empty() {
                    p { class: "toolbar-hint", "{i18n_text.no_links}" }
                } else {
                    div { class: "progress-links-list",
                        for (index , link) in links.iter().cloned().enumerate() {
                            div { class: "progress-link-row", key: "{index}",
                                a {
                                    class: "progress-link-item",
                                    href: "{link.url}",
                                    target: "_blank",
                                    rel: "noopener noreferrer",
                                    "{link.label}"
                                }
                                if !is_inherited {
                                    button {
                                        r#type: "button",
                                        class: "progress-field-remove",
                                        onclick: move |_| {
                                            let mut updated: Vec<ProgressLink> = item_store
                                                .read()
                                                .items
                                                .iter()
                                                .find(|i| i.id == item_id)
                                                .and_then(|i| {
                                                    i
                                                        .placements
                                                        .iter()
                                                        .find(|p| {
                                                            p.category_id == category_id
                                                                && p.subcategory_id == subcategory_id
                                                        })
                                                })
                                                .and_then(|p| p.progress.iter().find(|n| n.id == node_id))
                                                .map(|n| n.redirect_links.clone())
                                                .unwrap_or_default();
                                            if index < updated.len() {
                                                updated.remove(index);
                                            }
                                            item_store
                                                .write()
                                                .set_progress_node_links(
                                                    item_id,
                                                    category_id,
                                                    subcategory_id,
                                                    node_id,
                                                    updated,
                                                );
                                        },
                                        "×"
                                    }
                                }
                            }
                        }
                    }
                }
                div { class: "progress-links-add",
                    input {
                        r#type: "text",
                        placeholder: "{i18n_text.link_name_placeholder}",
                        value: "{new_label}",
                        oninput: move |event| new_label.set(event.value()),
                    }
                    input {
                        r#type: "text",
                        placeholder: "https://...",
                        value: "{new_url}",
                        oninput: move |event| new_url.set(event.value()),
                    }
                    button {
                        r#type: "button",
                        class: "secondary-btn",
                        onclick: move |_| {
                            let label = new_label.read().trim().to_string();
                            let url = new_url.read().trim().to_string();
                            if !label.is_empty() && !url.is_empty() {
                                let mut updated: Vec<ProgressLink> = item_store
                                    .read()
                                    .items
                                    .iter()
                                    .find(|i| i.id == item_id)
                                    .and_then(|i| {
                                        i
                                            .placements
                                            .iter()
                                            .find(|p| {
                                                p.category_id == category_id
                                                    && p.subcategory_id == subcategory_id
                                            })
                                    })
                                    .and_then(|p| p.progress.iter().find(|n| n.id == node_id))
                                    .map(|n| n.redirect_links.clone())
                                    .unwrap_or_default();
                                updated.push(ProgressLink { label, url });
                                item_store
                                    .write()
                                    .set_progress_node_links(
                                        item_id,
                                        category_id,
                                        subcategory_id,
                                        node_id,
                                        updated,
                                    );
                                new_label.set(String::new());
                                new_url.set(String::new());
                            }
                        },
                        "+"
                    }
                }
                div { class: "modal-actions",
                    button {
                        r#type: "button",
                        class: "primary-btn",
                        onclick: move |_| onclose.call(()),
                        "{i18n_text.close}"
                    }
                }
            }
        }
    }
}

/// Champs libres (label/valeur) d'un nœud feuille (Text ou Card), affichés
/// sous forme de "chips" supprimables, plus un mini-formulaire pour en
/// ajouter un. Pour un nœud "Card", affiche en plus un champ pour l'URL de
/// l'image. Relit toujours l'état actuel depuis `ItemStore` avant de
/// modifier plutôt que de se fier à `fields` (qui peut être périmé), pour
/// éviter tout risque d'écraser une modification concurrente.
#[component]
fn ProgressFieldsEditor(
    item_id: usize,
    category_id: usize,
    subcategory_id: usize,
    node_id: usize,
    kind: ProgressNodeKind,
    fields: Vec<ItemInfoField>,
    image_url: Option<String>,
) -> Element {
    let i18n_text = use_texts();
    let mut item_store = use_context::<Signal<ItemStore>>();
    let mut new_label = use_signal(String::new);
    let mut new_value = use_signal(String::new);

    let field_rows: Vec<(usize, String, String)> = fields
        .iter()
        .cloned()
        .enumerate()
        .map(|(index, field)| (index, field.label, field.value))
        .collect();

    rsx! {
        div { class: "progress-node-fields",
            if kind == ProgressNodeKind::Card {
                input {
                    r#type: "text",
                    class: "progress-field-input",
                    placeholder: "{i18n_text.image_url}",
                    value: image_url.clone().unwrap_or_default(),
                    oninput: move |event| {
                        let value = event.value();
                        let new_image = if value.trim().is_empty() { None } else { Some(value) };
                        item_store
                            .write()
                            .set_progress_node_image(
                                item_id,
                                category_id,
                                subcategory_id,
                                node_id,
                                new_image,
                            );
                    },
                }
            }
            for (index , label , value) in field_rows {
                span { class: "progress-field-chip", key: "{index}",
                    "{label} : {value}"
                    button {
                        r#type: "button",
                        class: "progress-field-remove",
                        onclick: move |_| {
                            let mut updated: Vec<ItemInfoField> = item_store
                                .read()
                                .items
                                .iter()
                                .find(|i| i.id == item_id)
                                .and_then(|i| {
                                    i
                                        .placements
                                        .iter()
                                        .find(|p| {
                                            p.category_id == category_id
                                                && p.subcategory_id == subcategory_id
                                        })
                                })
                                .and_then(|p| p.progress.iter().find(|n| n.id == node_id))
                                .map(|n| n.fields.clone())
                                .unwrap_or_default();
                            if index < updated.len() {
                                updated.remove(index);
                            }
                            item_store
                                .write()
                                .set_progress_node_fields(
                                    item_id,
                                    category_id,
                                    subcategory_id,
                                    node_id,
                                    updated,
                                );
                        },
                        "×"
                    }
                }
            }
            input {
                r#type: "text",
                class: "progress-field-input",
                placeholder: "{i18n_text.progress_field_label_placeholder}",
                value: "{new_label}",
                oninput: move |event| new_label.set(event.value()),
            }
            input {
                r#type: "text",
                class: "progress-field-input",
                placeholder: "{i18n_text.progress_field_value_placeholder}",
                value: "{new_value}",
                oninput: move |event| new_value.set(event.value()),
            }
            button {
                r#type: "button",
                class: "secondary-btn",
                onclick: move |_| {
                    let label = new_label.read().trim().to_string();
                    let value = new_value.read().trim().to_string();
                    if !label.is_empty() && !value.is_empty() {
                        let mut updated: Vec<ItemInfoField> = item_store
                            .read()
                            .items
                            .iter()
                            .find(|i| i.id == item_id)
                            .and_then(|i| {
                                i
                                    .placements
                                    .iter()
                                    .find(|p| {
                                        p.category_id == category_id
                                            && p.subcategory_id == subcategory_id
                                    })
                            })
                            .and_then(|p| p.progress.iter().find(|n| n.id == node_id))
                            .map(|n| n.fields.clone())
                            .unwrap_or_default();
                        updated.push(ItemInfoField { label, value });
                        item_store
                            .write()
                            .set_progress_node_fields(
                                item_id,
                                category_id,
                                subcategory_id,
                                node_id,
                                updated,
                            );
                        new_label.set(String::new());
                        new_value.set(String::new());
                    }
                },
                "+"
            }
        }
    }
}

/// Bouton "+" en forme de vignette qui crée directement une nouvelle carte
/// (avec un titre provisoire) et ouvre aussitôt sa popup de configuration.
#[component]
fn CardAddTile(item_id: usize, category_id: usize, subcategory_id: usize, parent_id: Option<usize>, editing_card: Signal<Option<usize>>) -> Element {
    let i18n_text = use_texts();
    let mut item_store = use_context::<Signal<ItemStore>>();

    rsx! {
        button {
            r#type: "button",
            class: "progress-card-tile progress-card-add",
            onclick: move |_| {
                let new_id = item_store
                    .write()
                    .add_progress_node(
                        item_id,
                        category_id,
                        subcategory_id,
                        parent_id,
                        ProgressNodeKind::Card,
                        None,
                        i18n_text.new_card_title.to_string(),
                    );
                if let Some(new_id) = new_id {
                    editing_card.set(Some(new_id));
                }
            },
            span { class: "progress-card-add-icon", "+" }
        }
    }
}

/// Formulaire d'ajout pour une liste "texte" : juste un titre.
#[component]
fn TextCreateForm(item_id: usize, category_id: usize, subcategory_id: usize, parent_id: Option<usize>) -> Element {
    let i18n_text = use_texts();
    let mut item_store = use_context::<Signal<ItemStore>>();
    let mut new_title = use_signal(String::new);

    rsx! {
        div { class: "progress-add-form",
            input {
                r#type: "text",
                class: "progress-add-input",
                placeholder: "{i18n_text.add_entry_placeholder}",
                value: "{new_title}",
                oninput: move |event| new_title.set(event.value()),
            }
            button {
                r#type: "button",
                class: "secondary-btn",
                onclick: move |_| {
                    let title = new_title.read().trim().to_string();
                    if !title.is_empty() {
                        item_store
                            .write()
                            .add_progress_node(
                                item_id,
                                category_id,
                                subcategory_id,
                                parent_id,
                                ProgressNodeKind::Text,
                                None,
                                title,
                            );
                        new_title.set(String::new());
                    }
                },
                "+"
            }
        }
    }
}

/// Formulaire de création d'une liste ("Group") : titre + choix du type de
/// SES enfants, décidé une fois pour toutes à la création (racine, ou
/// sous-liste imbriquée dans une liste elle-même de type "sous-listes").
#[component]
fn GroupCreateForm(item_id: usize, category_id: usize, subcategory_id: usize, parent_id: Option<usize>) -> Element {
    let i18n_text = use_texts();
    let mut item_store = use_context::<Signal<ItemStore>>();
    let mut new_title = use_signal(String::new);
    let mut new_children_kind = use_signal(|| ProgressNodeKind::Card);

    let placeholder = if parent_id.is_none() {
        i18n_text.list_name_placeholder
    } else {
        i18n_text.sublist_name_placeholder
    };

    rsx! {
        div { class: "progress-add-form",
            input {
                r#type: "text",
                class: "progress-add-input",
                placeholder: "{placeholder}",
                value: "{new_title}",
                oninput: move |event| new_title.set(event.value()),
            }
            select {
                class: "progress-add-kind",
                value: match *new_children_kind.read() {
                    ProgressNodeKind::Group => "group",
                    ProgressNodeKind::Text => "text",
                    ProgressNodeKind::Card => "card",
                },
                onchange: move |event| {
                    new_children_kind
                        .set(
                            match event.value().as_str() {
                                "group" => ProgressNodeKind::Group,
                                "text" => ProgressNodeKind::Text,
                                _ => ProgressNodeKind::Card,
                            },
                        );
                },
                option { value: "card", "{i18n_text.contains_cards}" }
                option { value: "text", "{i18n_text.contains_text}" }
                option { value: "group", "{i18n_text.contains_sublists}" }
            }
            button {
                r#type: "button",
                class: "secondary-btn",
                onclick: move |_| {
                    let title = new_title.read().trim().to_string();
                    if !title.is_empty() {
                        let children_kind = *new_children_kind.read();
                        item_store
                            .write()
                            .add_progress_node(
                                item_id,
                                category_id,
                                subcategory_id,
                                parent_id,
                                ProgressNodeKind::Group,
                                Some(children_kind),
                                title,
                            );
                        new_title.set(String::new());
                    }
                },
                "+"
            }
        }
    }
}