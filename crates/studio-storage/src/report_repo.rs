//! Report repository — the Markdown a report generation produced, and where it
//! was exported to.
//!
//! Reports are stored as rendered text rather than re-derived on demand: an export
//! has to stay reproducible after the analysis behind it has been recomputed or
//! invalidated, and `exported_path` records the file the user actually picked.

use chrono::{DateTime, Utc};
use duckdb::{params, Connection, OptionalExt};
use uuid::Uuid;

use crate::project_repo::{naive_to_utc, parse_uuid};
use crate::Result;

/// A stored report.
#[derive(Debug, Clone)]
pub struct StoredReport {
    pub report_id: Uuid,
    pub project_id: Uuid,
    pub title: Option<String>,
    pub markdown: String,
    pub exported_path: Option<String>,
    pub created_at: DateTime<Utc>,
}

const COLUMNS: &str = "report_id, project_id, title, markdown, exported_path, created_at";

/// Persist a generated report.
pub fn save_report(conn: &Connection, report: &StoredReport) -> Result<()> {
    conn.execute(
        "INSERT INTO reports (report_id, project_id, title, markdown, exported_path, created_at)
         VALUES (?, ?, ?, ?, ?, ?)",
        params![
            report.report_id.to_string(),
            report.project_id.to_string(),
            report.title,
            report.markdown,
            report.exported_path,
            report.created_at.to_rfc3339(),
        ],
    )
    .map_err(|e| crate::StudioError::Database(e.to_string()))?;
    Ok(())
}

/// The stored report with this id, if it exists.
pub fn get_report(conn: &Connection, report_id: Uuid) -> Result<Option<StoredReport>> {
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM reports WHERE report_id = ?"),
        [report_id.to_string()],
        row_to_report,
    )
    .optional()
    .map_err(|e| crate::StudioError::Database(e.to_string()))
}

/// The most recent report for a project — what the export button works from.
pub fn latest_report(conn: &Connection, project_id: Uuid) -> Result<Option<StoredReport>> {
    conn.query_row(
        &format!(
            "SELECT {COLUMNS} FROM reports WHERE project_id = ? \
             ORDER BY created_at DESC LIMIT 1"
        ),
        [project_id.to_string()],
        row_to_report,
    )
    .optional()
    .map_err(|e| crate::StudioError::Database(e.to_string()))
}

/// Record where a report was written, so the next export can default to the same
/// place and the user can tell what has already been saved.
pub fn mark_exported(conn: &Connection, report_id: Uuid, path: &str) -> Result<()> {
    conn.execute(
        "UPDATE reports SET exported_path = ? WHERE report_id = ?",
        params![path, report_id.to_string()],
    )
    .map_err(|e| crate::StudioError::Database(e.to_string()))?;
    Ok(())
}

fn row_to_report(row: &duckdb::Row<'_>) -> duckdb::Result<StoredReport> {
    let report_id: String = row.get(0)?;
    let project_id: String = row.get(1)?;
    let created_at: chrono::NaiveDateTime = row.get(5)?;
    Ok(StoredReport {
        report_id: parse_uuid(&report_id)?,
        project_id: parse_uuid(&project_id)?,
        title: row.get(2)?,
        markdown: row.get(3)?,
        exported_path: row.get(4)?,
        created_at: naive_to_utc(created_at),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migration::run_migrations;
    use crate::project_repo::create;
    use chrono::TimeZone;
    use studio_core::project::Project;

    fn conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    fn project(conn: &Connection) -> Uuid {
        create(conn, &Project::new("Report Project"))
            .unwrap()
            .project_id
    }

    fn report(project_id: Uuid, minute: u32) -> StoredReport {
        StoredReport {
            report_id: Uuid::new_v4(),
            project_id,
            title: Some(format!("报告 {minute}")),
            markdown: format!("# 报告 {minute}\n"),
            exported_path: None,
            created_at: Utc.with_ymd_and_hms(2026, 1, 1, 0, minute, 0).unwrap(),
        }
    }

    #[test]
    fn test_save_and_get_roundtrip() {
        let conn = conn();
        let pid = project(&conn);
        let stored = report(pid, 5);
        save_report(&conn, &stored).unwrap();

        let loaded = get_report(&conn, stored.report_id).unwrap().unwrap();
        assert_eq!(loaded.report_id, stored.report_id);
        assert_eq!(loaded.project_id, pid);
        assert_eq!(loaded.title.as_deref(), Some("报告 5"));
        assert_eq!(loaded.markdown, "# 报告 5\n");
        assert_eq!(loaded.exported_path, None);
        // Timestamps go in as UTC and come back as UTC, not shifted into the
        // machine's local zone.
        assert_eq!(loaded.created_at, stored.created_at);
    }

    #[test]
    fn test_missing_report_is_none() {
        let conn = conn();
        assert!(get_report(&conn, Uuid::new_v4()).unwrap().is_none());
        assert!(latest_report(&conn, Uuid::new_v4()).unwrap().is_none());
    }

    #[test]
    fn test_latest_report_is_the_newest() {
        let conn = conn();
        let pid = project(&conn);
        let old = report(pid, 1);
        let new = report(pid, 9);
        save_report(&conn, &old).unwrap();
        save_report(&conn, &new).unwrap();

        let latest = latest_report(&conn, pid).unwrap().unwrap();
        assert_eq!(latest.report_id, new.report_id);

        // A report belongs to exactly one project.
        let other = project(&conn);
        assert!(latest_report(&conn, other).unwrap().is_none());
    }

    #[test]
    fn test_mark_exported_records_the_path() {
        let conn = conn();
        let pid = project(&conn);
        let stored = report(pid, 3);
        save_report(&conn, &stored).unwrap();

        mark_exported(&conn, stored.report_id, "/tmp/report.md").unwrap();
        let loaded = get_report(&conn, stored.report_id).unwrap().unwrap();
        assert_eq!(loaded.exported_path.as_deref(), Some("/tmp/report.md"));
    }

    #[test]
    fn test_v12_reports_schema() {
        let conn = conn();
        let cols: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM information_schema.columns \
                 WHERE table_name = 'reports'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(cols, 6);

        // The UUID primary key rejects a second row with the same id.
        let pid = project(&conn);
        let mut a = report(pid, 1);
        a.report_id = Uuid::new_v4();
        let mut b = a.clone();
        b.markdown = "# duplicate\n".into();
        save_report(&conn, &a).unwrap();
        assert!(save_report(&conn, &b).is_err());
    }
}
