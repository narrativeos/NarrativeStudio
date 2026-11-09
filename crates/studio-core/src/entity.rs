//! Entity model (NER results from TraceView).

use serde::{Deserialize, Serialize};

/// Named entity category.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EntityCategory {
    Person,
    Organization,
    Location,
    Date,
    Facility,
    Product,
    Number,
    #[default]
    Unknown,
}

impl EntityCategory {
    /// Convert to string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Person => "PERSON",
            Self::Organization => "ORGANIZATION",
            Self::Location => "LOCATION",
            Self::Date => "DATE",
            Self::Facility => "FACILITY",
            Self::Product => "PRODUCT",
            Self::Number => "NUMBER",
            Self::Unknown => "UNKNOWN",
        }
    }
}

impl std::fmt::Display for EntityCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for EntityCategory {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "PERSON" => Ok(Self::Person),
            "ORGANIZATION" => Ok(Self::Organization),
            "LOCATION" => Ok(Self::Location),
            "DATE" => Ok(Self::Date),
            "FACILITY" => Ok(Self::Facility),
            "PRODUCT" => Ok(Self::Product),
            "NUMBER" => Ok(Self::Number),
            _ => Ok(Self::Unknown),
        }
    }
}

/// A named entity extracted from text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub text: String,
    pub category: EntityCategory,
    pub confidence: f32,
    pub source: String,
    pub keep: bool,
    pub filter: Option<String>,
    pub filter_reason: Option<String>,
    pub span: (usize, usize),
    /// Content-derived identity issued by the entity registry
    /// (`studio_storage::entity_registry`). Stable across imports and
    /// projects, unlike the per-import `entity_id` row key. `None` for rows
    /// written before the registry existed (v13) and for entities that were
    /// never persisted.
    #[serde(default)]
    pub stable_id: Option<String>,
}

/// Aggregated view of one identity across a whole project: what the
/// knowsphere exporter builds surface-layer nodes from. Produced by
/// `studio_storage::entity_stats_for_project`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityStat {
    pub stable_id: String,
    pub text: String,
    /// `EntityCategory::as_str()` value (`PERSON`, `LOCATION`, …).
    pub category: String,
    pub mentions: u64,
    /// Highest confidence seen for this identity.
    pub confidence: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_category_roundtrip() {
        let cat = EntityCategory::Person;
        assert_eq!(cat.as_str(), "PERSON");
        let parsed: EntityCategory = "PERSON".parse().unwrap();
        assert_eq!(cat, parsed);
    }

    #[test]
    fn test_entity_category_unknown() {
        let cat: EntityCategory = "SOMETHING_ELSE".parse().unwrap();
        assert_eq!(cat, EntityCategory::Unknown);
    }

    #[test]
    fn test_entity_serde() {
        let e = Entity {
            text: "Marlowe".to_string(),
            category: EntityCategory::Unknown,
            confidence: 0.55,
            source: "pos/nnp".to_string(),
            keep: true,
            filter: None,
            filter_reason: None,
            span: (12, 19),
            stable_id: None,
        };
        let json = serde_json::to_string(&e).unwrap();
        let deserialized: Entity = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.text, "Marlowe");
        assert_eq!(deserialized.category, EntityCategory::Unknown);
    }
}
