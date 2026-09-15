//! Project model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A NarrativeStudio project representing one TraceView analysis output.
///
/// TraceView 项目文件夹结构:
/// ```text
/// {source_path}/
/// ├── project.json              # 项目元信息 (ProjectName, PdfPath, CreatedAt...)
/// ├── mineru/                   # PDF 原始解析 (content_list, layout, model, markdown)
/// │   └── {book_name}/hybrid_auto/
/// │       ├── {book}_content_list_v2.json
/// │       ├── {book}_middle.json
/// │       ├── {book}_model.json
/// │       ├── {book}.md
/// │       └── images/
/// ├── popo/                     # 文档结构/目录分析
/// │   └── popo_result.json      # TOC tree + 376 目录条目
/// └── semantic/                 # NLP 语义分析
///     ├── semantic_result.json  # blocks + tokens + entities + relations
///     ├── enriched_result.json  # 增强版 blocks
///     ├── term_result.json      # 术语/关系/术语树
///     ├── locations.geojson
///     └── locations.geolibre.json
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub project_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub genre: Option<String>,
    pub language: String,
    /// TraceView 项目文件夹路径（持久化依据，用于定位所有源文件）
    pub source_path: Option<String>,
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
    pub source_path: Option<String>,
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
            source_path: None,
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
