//! Entity model (NER results from TraceView).

use serde::{Deserialize, Serialize};

/// Named entity category.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EntityCategory {
    Person,
    Organization,
    Location,
    Date,
    Facility,
    Product,
    Number,
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
        };
        let json = serde_json::to_string(&e).unwrap();
        let deserialized: Entity = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.text, "Marlowe");
        assert_eq!(deserialized.category, EntityCategory::Unknown);
    }
}
