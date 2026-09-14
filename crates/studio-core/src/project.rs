//! Project model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A NarrativeStudio project representing one TraceView analysis output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub project_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub genre: Option<String>,
    pub language: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Lightweight project summary for list views.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub project_id: Uuid,
    pub name: String,
    pub genre: Option<String>,
    pub word_count: Option<u64>,
    pub chapter_count: Option<u32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Project {
    /// Create a new project with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            project_id: Uuid::new_v4(),
            name: name.into(),
            description: None,
            genre: None,
            language: "zh-CN".to_string(),
            created_at: now,
            updated_at: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_new() {
        let p = Project::new("Test Project");
        assert_eq!(p.name, "Test Project");
        assert_eq!(p.language, "zh-CN");
        assert!(p.description.is_none());
        assert_eq!(p.created_at, p.updated_at);
    }

    #[test]
    fn test_project_serde_roundtrip() {
        let p = Project::new("Serialize Me");
        let json = serde_json::to_string(&p).unwrap();
        let deserialized: Project = serde_json::from_str(&json).unwrap();
        assert_eq!(p.project_id, deserialized.project_id);
        assert_eq!(p.name, deserialized.name);
    }
}
