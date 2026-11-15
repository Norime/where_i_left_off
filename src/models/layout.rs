use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

/// Position du menu principal (et de celui des réglages). D'autres
/// positions (en haut…) pourront s'ajouter : un variant, une ligne dans
/// `ALL`, `key`, la classe CSS et son libellé (`Texts::menu_position_label`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MenuPosition {
    #[default]
    Left,
    Right,
}

impl MenuPosition {
    /// Ordre d'affichage dans les réglages.
    pub const ALL: [MenuPosition; 2] = [MenuPosition::Left, MenuPosition::Right];

    /// Clé enregistrée dans le compte et les fichiers de thème.
    pub fn key(self) -> &'static str {
        match self {
            MenuPosition::Left => "left",
            MenuPosition::Right => "right",
        }
    }

    pub fn from_key(key: &str) -> Option<MenuPosition> {
        MenuPosition::ALL.into_iter().find(|p| p.key() == key)
    }
}

/// Mise en page de l'application, propre au compte et indépendante du
/// thème (elle peut seulement être exportée avec lui). Chaque réglage
/// ajoute une classe sur l'enveloppe de l'application (`root_class`), que
/// les feuilles de style utilisent pour placer les éléments.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Layout {
    pub menu_position: MenuPosition,
}

impl Layout {
    /// Classes à poser sur l'enveloppe de l'application.
    pub fn root_class(&self) -> String {
        format!("app-layout layout-menu-{}", self.menu_position.key())
    }

    /// Valeur enregistrée (compte, fichier de thème).
    pub fn to_value(&self) -> Value {
        serde_json::json!({ "menu_position": self.menu_position.key() })
    }

    /// Lecture tolérante : un réglage absent, inconnu (retiré de
    /// l'application) ou invalide garde sa valeur par défaut.
    pub fn from_value(value: &Value) -> Layout {
        let default = Layout::default();
        Layout {
            menu_position: value
                .get("menu_position")
                .and_then(Value::as_str)
                .and_then(MenuPosition::from_key)
                .unwrap_or(default.menu_position),
        }
    }
}

impl Serialize for Layout {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_value().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Layout {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Layout::from_value(&Value::deserialize(deserializer)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_round_trips_and_reads_leniently() {
        let layout = Layout { menu_position: MenuPosition::Right };
        let text = serde_json::to_string(&layout).unwrap();
        assert_eq!(serde_json::from_str::<Layout>(&text).unwrap(), layout);
        let unknown: Layout = serde_json::from_str(r#"{ "menu_position": "diagonal" }"#).unwrap();
        assert_eq!(unknown, Layout::default());
        let garbage: Layout = serde_json::from_str("42").unwrap();
        assert_eq!(garbage, Layout::default());
    }
}
