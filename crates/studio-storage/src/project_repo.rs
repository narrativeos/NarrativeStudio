//! Project repository — CRUD operations for projects.

use duckdb::{types::Type, Connection, Error as DuckError, Transaction};
use studio_core::project::{Project, ProjectSummary};
use uuid::Uuid;

use crate::Result;

fn parse_uuid(s: &str) -> std::result::Result<Uuid, DuckError> {
    Uuid::parse_str(s).map_err(|e| {
        DuckError::FromSqlConversionFailure(
            0,
            Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            )),
        )
    })
}

fn naive_to_utc(dt: chrono::NaiveDateTime) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::from_naive_utc_and_offset(dt, chrono::Utc)
}

/// Create a new project and return it.
pub fn create(conn: &Connection, project: &Project) -> Result<Project> {
    conn.execute(
        "INSERT INTO projects (project_id, name, description, genre, language, source_path, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        duckdb::params![
            project.project_id.to_string(),
            project.name,
            project.description,
            project.genre,
            project.language,
            project.source_path,
            project.created_at.to_rfc3339(),
            project.updated_at.to_rfc3339(),
        ],
    )
    .map_err(|e| crate::StudioError::Database(e.to_string()))?;
    Ok(project.clone())
}

/// Get a project by ID.
pub fn get(conn: &Connection, project_id: Uuid) -> Result<Project> {
    conn.query_row(
        "SELECT project_id, name, description, genre, language, source_path, created_at, updated_at
         FROM projects WHERE project_id = ?",
        [project_id.to_string()],
        |row| {
            let pid: String = row.get(0)?;
            let name: String = row.get(1)?;
            let description: Option<String> = row.get(2)?;
            let genre: Option<String> = row.get(3)?;
            let language: String = row.get(4)?;
            let source_path: Option<String> = row.get(5)?;
            let created_at: chrono::NaiveDateTime = row.get(6)?;
            let updated_at: chrono::NaiveDateTime = row.get(7)?;

            Ok(Project {
                project_id: parse_uuid(&pid)?,
                name,
                description,
                genre,
                language,
                source_path,
                created_at: naive_to_utc(created_at),
                updated_at: naive_to_utc(updated_at),
            })
        },
    )
    .map_err(|e| crate::StudioError::Database(format!("Project not found: {e}")))
}

/// List all projects.
pub fn list(conn: &Connection) -> Result<Vec<ProjectSummary>> {
    let mut stmt = conn
        .prepare(
            "SELECT p.project_id, p.name, p.genre, p.source_path, p.created_at, p.updated_at,
                    (SELECT COUNT(*) FROM documents d WHERE d.project_id = p.project_id)
             FROM projects p
             ORDER BY p.updated_at DESC",
        )
        .map_err(|e| crate::StudioError::Database(e.to_string()))?;

    let rows = stmt
        .query_map([], |row| {
            let pid: String = row.get(0)?;
            let name: String = row.get(1)?;
            let genre: Option<String> = row.get(2)?;
            let source_path: Option<String> = row.get(3)?;
            let created_at: chrono::NaiveDateTime = row.get(4)?;
            let updated_at: chrono::NaiveDateTime = row.get(5)?;
            let doc_count: i64 = row.get(6)?;

            Ok(ProjectSummary {
                project_id: parse_uuid(&pid)?,
                name,
                genre,
                word_count: None,
                chapter_count: Some(doc_count as u32),
                source_path,
                created_at: naive_to_utc(created_at),
                updated_at: naive_to_utc(updated_at),
            })
        })
        .map_err(|e| crate::StudioError::Database(e.to_string()))?;

    let mut projects = Vec::new();
    for row in rows {
        projects.push(row.map_err(|e| crate::StudioError::Database(e.to_string()))?);
    }
    Ok(projects)
}

/// Delete a project by ID, cascading to all of its associated data.
///
/// Removes, in dependency order (children first), the project's analysis
/// results and concerns, its documents and every imported block/token/entity/
/// noun-signal row, and finally the project row itself. Runs in a single
/// transaction so a failure leaves the database consistent.
pub fn delete(conn: &Connection, project_id: Uuid) -> Result<()> {
    let pid = project_id.to_string();
    let tx = Transaction::new_unchecked(conn)
        .map_err(|e| crate::StudioError::Database(e.to_string()))?;

    // Analysis data (children first): concerns -> analysis_results.
    tx.execute(
        "DELETE FROM concerns WHERE result_id IN \
         (SELECT result_id FROM analysis_results WHERE project_id = ?)",
        [pid.as_str()],
    )
    .map_err(|e| crate::StudioError::Database(e.to_string()))?;
    tx.execute(
        "DELETE FROM analysis_results WHERE project_id = ?",
        [pid.as_str()],
    )
    .map_err(|e| crate::StudioError::Database(e.to_string()))?;

    // Imported data (children first): tokens/entities/noun_signals ->
    // semantic_blocks -> documents, all scoped to this project's blocks.
    let project_blocks = "SELECT source_block_id FROM semantic_blocks \
                          WHERE doc_id IN (SELECT doc_id FROM documents WHERE project_id = ?)";
    for table in ["noun_signals", "entities", "tokens"] {
        tx.execute(
            &format!("DELETE FROM {table} WHERE source_block_id IN ({project_blocks})"),
            [pid.as_str()],
        )
        .map_err(|e| crate::StudioError::Database(e.to_string()))?;
    }
    tx.execute(
        "DELETE FROM semantic_blocks WHERE doc_id IN \
         (SELECT doc_id FROM documents WHERE project_id = ?)",
        [pid.as_str()],
    )
    .map_err(|e| crate::StudioError::Database(e.to_string()))?;
    tx.execute("DELETE FROM documents WHERE project_id = ?", [pid.as_str()])
        .map_err(|e| crate::StudioError::Database(e.to_string()))?;

    // Finally the project row itself.
    tx.execute("DELETE FROM projects WHERE project_id = ?", [pid.as_str()])
        .map_err(|e| crate::StudioError::Database(e.to_string()))?;

    tx.commit().map_err(|e| crate::StudioError::Database(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migration::run_migrations;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    #[test]
    fn test_create_and_get() {
        let conn = test_conn();
        let project = Project::new("Test Novel");
        create(&conn, &project).unwrap();
        let fetched = get(&conn, project.project_id).unwrap();
        assert_eq!(fetched.name, "Test Novel");
    }

    #[test]
    fn test_list_empty() {
        let conn = test_conn();
        let projects = list(&conn).unwrap();
        assert!(projects.is_empty());
    }

    #[test]
    fn test_list_with_projects() {
        let conn = test_conn();
        create(&conn, &Project::new("Novel A")).unwrap();
        create(&conn, &Project::new("Novel B")).unwrap();
        let projects = list(&conn).unwrap();
        assert_eq!(projects.len(), 2);
    }

    #[test]
    fn test_delete() {
        let conn = test_conn();
        let project = Project::new("To Delete");
        create(&conn, &project).unwrap();
        delete(&conn, project.project_id).unwrap();
        let projects = list(&conn).unwrap();
        assert!(projects.is_empty());
    }

    #[test]
    fn test_delete_cascades() {
        use crate::save_document;
        use studio_core::document::{DocumentData, SemanticBlock, Token};
        use studio_core::entity::{Entity, EntityCategory};
        use studio_core::noun_signal::NounSignal;

        let conn = test_conn();
        let project = create(&conn, &Project::new("Cascade")).unwrap();

        let doc = DocumentData {
            doc_id: Uuid::new_v4(),
            title: "Doc".into(),
            blocks: vec![SemanticBlock {
                source_block_id: "blk-1".into(),
                content: "Hello".into(),
                section_path: "Ch1".into(),
                block_type: "paragraph".into(),
                title: None,
                tokens: vec![Token {
                    text: "Hello".into(),
                    pos: "NNP".into(),
                    confidence: 0.99,
                    span: (0, 5),
                    source: "test".into(),
                }],
                entities: vec![Entity {
                    text: "Hello".into(),
                    category: EntityCategory::Person,
                    confidence: 0.9,
                    source: "test".into(),
                    keep: true,
                    filter: None,
                    filter_reason: None,
                    span: (0, 5),
                }],
                noun_signals: vec![NounSignal {
                    text: "Hello".into(),
                    pos: "NNP".into(),
                    syntactic_role: Some("Subject".into()),
                    score: 0.8,
                    span: (0, 5),
                    evidence: None,
                }],
            }],
            total_word_count: 1,
            total_char_count: 5,
        };
        save_document(&conn, project.project_id, &doc).unwrap();

        // Sanity: the imported data exists before the delete.
        let blocks: i64 = conn
            .query_row("SELECT COUNT(*) FROM semantic_blocks", [], |r| r.get(0))
            .unwrap();
        assert_eq!(blocks, 1);

        delete(&conn, project.project_id).unwrap();

        // Every table that references the project is now empty (no orphans).
        for table in [
            "projects",
            "documents",
            "semantic_blocks",
            "tokens",
            "entities",
            "noun_signals",
            "analysis_results",
            "concerns",
        ] {
            let n: i64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
                .unwrap();
            assert_eq!(n, 0, "expected {table} to be empty after delete");
        }
    }
}
