//! Concern (issue/point of concern) model.

use serde::{Deserialize, Serialize};

/// Severity level for a concern.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// Location of a concern within the document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConcernLocation {
    pub block_id: Option<String>,
    pub section_path: Option<String>,
    pub span_start: Option<usize>,
    pub span_end: Option<usize>,
}

/// A specific issue or point of concern identified during analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concern {
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub location: ConcernLocation,
    pub suggestion: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_concern_serde() {
        let c = Concern {
            severity: Severity::Medium,
            title: "Abrupt character transformation".to_string(),
            description: "Dan's transformation feels too abrupt".to_string(),
            location: ConcernLocation {
                block_id: Some("block_5".to_string()),
                section_path: Some("Chapter 3".to_string()),
                span_start: Some(100),
                span_end: Some(250),
            },
            suggestion: Some("Add gradual development".to_string()),
        };
        let json = serde_json::to_string(&c).unwrap();
        let deserialized: Concern = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.severity, Severity::Medium);
        assert_eq!(deserialized.title, "Abrupt character transformation");
    }
}
