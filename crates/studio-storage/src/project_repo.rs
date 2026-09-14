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

/// Repository for project CRUD operations.
pub struct ProjectRepo {
    conn: Connection,
}

impl ProjectRepo {
    /// Create a new ProjectRepo with the given connection.
    pub fn new(conn: Connection) -> Self {
        Self { conn }
    }

    /// Create a new project and return it.
    pub fn create(&self, project: &Project) -> Result<Project> {
        self.conn
            .execute(
                "INSERT INTO projects (project_id, name, description, genre, language, created_at, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
                duckdb::params![
                    project.project_id.to_string(),
                    project.name,
                    project.description,
                    project.genre,
                    project.language,
                    project.created_at.to_rfc3339(),
                    project.updated_at.to_rfc3339(),
                ],
            )
            .map_err(|e| crate::StudioError::Database(e.to_string()))?;
        Ok(project.clone())
    }

    /// Get a project by ID.
    pub fn get(&self, project_id: Uuid) -> Result<Project> {
        self.conn
            .query_row(
                "SELECT project_id, name, description, genre, language, created_at, updated_at
                 FROM projects WHERE project_id = ?",
                [project_id.to_string()],
                |row| {
                    let pid: String = row.get(0)?;
                    let name: String = row.get(1)?;
                    let description: Option<String> = row.get(2)?;
                    let genre: Option<String> = row.get(3)?;
                    let language: String = row.get(4)?;
                    let created_at: chrono::NaiveDateTime = row.get(5)?;
                    let updated_at: chrono::NaiveDateTime = row.get(6)?;

                    Ok(Project {
                        project_id: parse_uuid(&pid)?,
                        name,
                        description,
                        genre,
                        language,
                        created_at: naive_to_utc(created_at),
                        updated_at: naive_to_utc(updated_at),
                    })
                },
            )
            .map_err(|e| crate::StudioError::Database(format!("Project not found: {e}")))
    }

    /// List all projects.
    pub fn list(&self) -> Result<Vec<ProjectSummary>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT p.project_id, p.name, p.genre, p.created_at, p.updated_at,
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
                let created_at: chrono::NaiveDateTime = row.get(3)?;
                let updated_at: chrono::NaiveDateTime = row.get(4)?;
                let doc_count: i64 = row.get(5)?;

                Ok(ProjectSummary {
                    project_id: parse_uuid(&pid)?,
                    name,
                    genre,
                    word_count: None,
                    chapter_count: Some(doc_count as u32),
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
    pub fn delete(&self, project_id: Uuid) -> Result<()> {
        self.conn
            .execute(
                "DELETE FROM projects WHERE project_id = ?",
                [project_id.to_string()],
            )
            .map_err(|e| crate::StudioError::Database(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migration::run_migrations;

    fn test_repo() -> ProjectRepo {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        ProjectRepo::new(conn)
    }

    #[test]
    fn test_create_and_get() {
        let repo = test_repo();
        let project = Project::new("Test Novel");
        repo.create(&project).unwrap();
        let fetched = repo.get(project.project_id).unwrap();
        assert_eq!(fetched.name, "Test Novel");
    }

    #[test]
    fn test_list_empty() {
        let repo = test_repo();
        let projects = repo.list().unwrap();
        assert!(projects.is_empty());
    }

    #[test]
    fn test_list_with_projects() {
        let repo = test_repo();
        repo.create(&Project::new("Novel A")).unwrap();
        repo.create(&Project::new("Novel B")).unwrap();
        let projects = repo.list().unwrap();
        assert_eq!(projects.len(), 2);
    }

    #[test]
    fn test_delete() {
        let repo = test_repo();
        let project = Project::new("To Delete");
        repo.create(&project).unwrap();
        repo.delete(project.project_id).unwrap();
        let projects = repo.list().unwrap();
        assert!(projects.is_empty());
    }
}
