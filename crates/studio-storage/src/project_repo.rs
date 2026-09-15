//! Project repository — CRUD operations for projects.

use duckdb::{types::Type, Connection, Error as DuckError};
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

/// Delete a project by ID.
pub fn delete(conn: &Connection, project_id: Uuid) -> Result<()> {
    conn.execute(
        "DELETE FROM projects WHERE project_id = ?",
        [project_id.to_string()],
    )
    .map_err(|e| crate::StudioError::Database(e.to_string()))?;
    Ok(())
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
}
