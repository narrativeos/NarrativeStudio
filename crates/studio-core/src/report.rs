//! Report model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A generated analysis report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub report_id: Uuid,
    pub project_id: Uuid,
    pub title: Option<String>,
    pub content: serde_json::Value,
    pub exported_path: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Report {
    /// Create a new report for the given project.
    pub fn new(project_id: Uuid, content: serde_json::Value) -> Self {
        Self {
            report_id: Uuid::new_v4(),
            project_id,
            title: None,
            content,
            exported_path: None,
            created_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_report_new() {
        let project_id = Uuid::new_v4();
        let report = Report::new(project_id, serde_json::json!({"score": 85}));
        assert_eq!(report.project_id, project_id);
        assert!(report.exported_path.is_none());
    }
}
