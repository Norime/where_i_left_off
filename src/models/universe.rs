use serde::{Deserialize, Serialize};

use crate::models::{Item, ItemInfoField, ItemPlacement, ItemStatus};

/// Un univers (MCU, Star Wars…) : des références vers des fiches
/// existantes, toutes catégories confondues, sans les copier, rangées selon
/// plusieurs ordres (sortie, chronologique…). Une fiche peut appartenir à
/// plusieurs univers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Universe {
    pub id: usize,
    pub name: String,
    #[serde(default)]
    pub entries: Vec<UniverseEntry>,
    #[serde(default)]
    pub orders: Vec<UniverseOrder>,
    /// Id de l'ordre affiché (le premier s'il n'existe plus).
    #[serde(default)]
    pub selected_order: usize,
}

/// Une œuvre de l'univers : un placement précis d'une fiche (le film dans
/// Vidéo › Films), dont le statut fait foi.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UniverseEntry {
    pub id: usize,
    pub item_id: usize,
    pub category_id: usize,
    pub subcategory_id: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum UniverseOrderKind {
    /// Trié automatiquement par date de sortie (info globale de l'œuvre).
    Release,
    /// Ordre libre : ids d'entrées.
    Manual(Vec<usize>),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UniverseOrder {
    pub id: usize,
    pub name: String,
    pub kind: UniverseOrderKind,
}

/// Date de sortie triable (année, mois, jour ; 0 si inconnu).
pub type ReleaseDate = (i32, u32, u32);

/// Lit une date de sortie écrite à la main ou par une source : « 2019 »,
/// « 2019-04-24 », « 2019-04 », « 24/04/2019 », ou à défaut la première
/// année plausible du texte.
pub fn parse_release_date(text: &str) -> Option<ReleaseDate> {
    let numbers: Vec<&str> = text.split(|c: char| !c.is_ascii_digit()).filter(|n| !n.is_empty()).collect();
    let number = |i: usize| numbers.get(i).and_then(|n| n.parse::<u32>().ok());
    let year_ok = |y: u32| (1800..=2300).contains(&y);
    let first = numbers.first()?;
    if first.len() == 4 {
        let year = number(0).filter(|y| year_ok(*y))?;
        let month = number(1).filter(|m| (1..=12).contains(m)).unwrap_or(0);
        let day = if month > 0 { number(2).filter(|d| (1..=31).contains(d)).unwrap_or(0) } else { 0 };
        return Some((year as i32, month, day));
    }
    // Jour / mois / année.
    if let (Some(day), Some(month), Some(year)) = (number(0), number(1), number(2)) {
        if year_ok(year) && (1..=12).contains(&month) && (1..=31).contains(&day) {
            return Some((year as i32, month, day));
        }
    }
    numbers
        .iter()
        .filter(|n| n.len() == 4)
        .filter_map(|n| n.parse::<u32>().ok())
        .find(|y| year_ok(*y))
        .map(|y| (y as i32, 0, 0))
}

/// Date de sortie d'une œuvre, d'après ses infos globales : un champ dont
/// le libellé parle de date, de sortie ou d'année, dans n'importe quelle
/// langue.
pub fn item_release_date(fields: &[ItemInfoField]) -> Option<ReleaseDate> {
    fields.iter().filter(|f| is_release_label(&f.label)).find_map(|f| parse_release_date(&f.value))
}

/// Valeur écrite de la date de sortie (« 2019-04-24 », « 2012 »).
pub fn item_release_label(fields: &[ItemInfoField]) -> Option<&str> {
    fields
        .iter()
        .find(|f| is_release_label(&f.label) && parse_release_date(&f.value).is_some())
        .map(|f| f.value.as_str())
}

fn is_release_label(label: &str) -> bool {
    const WORDS: [&str; 7] = ["sortie", "release", "date", "année", "year", "publication", "parution"];
    let label = label.to_lowercase();
    WORDS.iter().any(|word| label.contains(word))
}

impl Universe {
    pub fn new(id: usize, name: String, release_name: String, manual_name: String) -> Self {
        Universe {
            id,
            name,
            entries: Vec::new(),
            orders: vec![
                UniverseOrder { id: 0, name: release_name, kind: UniverseOrderKind::Release },
                UniverseOrder { id: 1, name: manual_name, kind: UniverseOrderKind::Manual(Vec::new()) },
            ],
            selected_order: 0,
        }
    }

    /// Ordre affiché : celui choisi, sinon le premier.
    pub fn current_order(&self) -> Option<&UniverseOrder> {
        self.orders.iter().find(|o| o.id == self.selected_order).or(self.orders.first())
    }

    pub fn contains(&self, item_id: usize, category_id: usize, subcategory_id: usize) -> bool {
        self.entries
            .iter()
            .any(|e| e.item_id == item_id && e.category_id == category_id && e.subcategory_id == subcategory_id)
    }

    /// Entrées rangées selon `order`. Ordre de sortie : sans date en fin de
    /// liste, à date égale dans l'ordre d'ajout. Ordre libre : les entrées
    /// absentes de la liste (ajoutées depuis) se placent à la fin.
    pub fn ordered_entries(&self, order: &UniverseOrder, items: &[Item]) -> Vec<UniverseEntry> {
        match &order.kind {
            UniverseOrderKind::Release => {
                let mut entries = self.entries.clone();
                entries.sort_by_key(|entry| {
                    let date = items.iter().find(|i| i.id == entry.item_id).and_then(|i| item_release_date(&i.extra_info));
                    (date.is_none(), date)
                });
                entries
            }
            UniverseOrderKind::Manual(ids) => {
                let mut entries: Vec<UniverseEntry> =
                    ids.iter().filter_map(|id| self.entries.iter().find(|e| e.id == *id).cloned()).collect();
                entries.extend(self.entries.iter().filter(|e| !ids.contains(&e.id)).cloned());
                entries
            }
        }
    }
}

/// Avancement d'un univers dans son ordre affiché : œuvres terminées,
/// total, et prochaine œuvre (la première ni terminée ni abandonnée).
#[derive(Clone, Debug, PartialEq)]
pub struct UniverseProgress {
    pub done: usize,
    pub total: usize,
    pub next: Option<UniverseEntry>,
    /// Au moins une œuvre terminée, en cours ou en pause.
    pub started: bool,
}

impl Universe {
    pub fn progress(&self, items: &[Item]) -> UniverseProgress {
        let ordered = self.current_order().map(|order| self.ordered_entries(order, items)).unwrap_or_default();
        let statuses: Vec<(UniverseEntry, ItemStatus)> = ordered
            .into_iter()
            .filter_map(|entry| {
                let status = entry_placement(&entry, items)?.1.status;
                Some((entry, status))
            })
            .collect();
        UniverseProgress {
            done: statuses.iter().filter(|(_, s)| *s == ItemStatus::Done).count(),
            total: statuses.len(),
            next: statuses
                .iter()
                .find(|(_, s)| !matches!(s, ItemStatus::Done | ItemStatus::Dropped))
                .map(|(e, _)| e.clone()),
            started: statuses
                .iter()
                .any(|(_, s)| matches!(s, ItemStatus::Done | ItemStatus::InProgress | ItemStatus::OnHold)),
        }
    }
}

/// Placement suivi par une entrée, s'il existe encore.
pub fn entry_placement<'a>(entry: &UniverseEntry, items: &'a [Item]) -> Option<(&'a Item, &'a ItemPlacement)> {
    let item = items.iter().find(|i| i.id == entry.item_id)?;
    let placement = item
        .placements
        .iter()
        .find(|p| p.category_id == entry.category_id && p.subcategory_id == entry.subcategory_id)?;
    Some((item, placement))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ItemOrientation, ItemPlacement};

    fn item(id: usize, date: Option<&str>) -> Item {
        Item {
            id,
            title: format!("{id}"),
            description: String::new(),
            placements: vec![ItemPlacement::new(1, 1)],
            orientation: ItemOrientation::Vertical,
            image_url: None,
            extra_info: date
                .map(|d| vec![ItemInfoField { label: "Date de sortie".to_string(), value: d.to_string() }])
                .unwrap_or_default(),
        }
    }

    #[test]
    fn release_dates_are_read_from_many_formats() {
        assert_eq!(parse_release_date("2019"), Some((2019, 0, 0)));
        assert_eq!(parse_release_date("2019-04-24"), Some((2019, 4, 24)));
        assert_eq!(parse_release_date("24/04/2019"), Some((2019, 4, 24)));
        assert_eq!(parse_release_date("Sorti en mai 2012"), Some((2012, 0, 0)));
        assert_eq!(parse_release_date("bientôt"), None);
    }

    #[test]
    fn release_order_sorts_by_date_and_puts_unknown_last() {
        let items = vec![item(0, None), item(1, Some("2012-05-04")), item(2, Some("2008")), item(3, Some("2012-04-25"))];
        let mut universe = Universe::new(0, "MCU".into(), "Sortie".into(), "Chrono".into());
        for (id, item) in items.iter().enumerate() {
            universe.entries.push(UniverseEntry { id, item_id: item.id, category_id: 1, subcategory_id: 1 });
        }
        let release = universe.orders[0].clone();
        let order: Vec<usize> = universe.ordered_entries(&release, &items).iter().map(|e| e.item_id).collect();
        assert_eq!(order, vec![2, 3, 1, 0]);
    }

    #[test]
    fn manual_order_appends_missing_entries() {
        let items = vec![item(0, None), item(1, None), item(2, None)];
        let mut universe = Universe::new(0, "MCU".into(), "Sortie".into(), "Chrono".into());
        for id in 0..3 {
            universe.entries.push(UniverseEntry { id, item_id: id, category_id: 1, subcategory_id: 1 });
        }
        universe.orders[1].kind = UniverseOrderKind::Manual(vec![2, 0]);
        let manual = universe.orders[1].clone();
        let order: Vec<usize> = universe.ordered_entries(&manual, &items).iter().map(|e| e.item_id).collect();
        assert_eq!(order, vec![2, 0, 1]);
    }

    #[test]
    fn progress_skips_dropped_works_for_next() {
        let mut items = vec![item(0, None), item(1, None), item(2, None)];
        items[0].placements[0].status = ItemStatus::Done;
        items[1].placements[0].status = ItemStatus::Dropped;
        let mut universe = Universe::new(0, "MCU".into(), "Sortie".into(), "Chrono".into());
        for id in 0..3 {
            universe.entries.push(UniverseEntry { id, item_id: id, category_id: 1, subcategory_id: 1 });
        }
        universe.selected_order = 1;
        let progress = universe.progress(&items);
        assert_eq!((progress.done, progress.total, progress.started), (1, 3, true));
        assert_eq!(progress.next.map(|e| e.item_id), Some(2));
    }
}
