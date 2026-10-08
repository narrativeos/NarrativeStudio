//! Analysis result repository — persists computed analysis dimensions so that
//! re-opening a project does not recompute them, and keeps the concerns
//! (recommendations) that a result produced.
//!
//! Rows are keyed by `(project_id, dimension, input_hash)`. `input_hash` is the
//! SHA-256 fingerprint of the content the analysis consumed
//! (`studio_analysis::document_hash`), so a stored row is only ever returned for
//! the exact input that produced it: a document that changed simply misses, and
//! the fresh result is stored under the new hash. Nothing here needs to know how
//! to invalidate a project when its text is edited — the key does that by
//! construction.

use duckdb::{params, Connection, OptionalExt};
use serde::de::DeserializeOwned;
use studio_core::concern::{Concern, ConcernLocation, Severity};
use uuid::Uuid;

use crate::Result;

/// One persisted analysis dimension, as stored in `analysis_results`.
#[derive(Debug, Clone)]
pub struct StoredResult {
    pub result_id: i64,
    pub dimension: String,
    pub tier: String,
    pub input_hash: String,
    pub score: Option<f32>,
    /// Serialized output of the analysis type that produced this row.
    pub data: String,
}

/// Store the output of `dimension` for `project_id` and return its row id.
///
/// Any row previously stored for the same `(project, dimension, input_hash)` is
/// replaced first: re-analysing unchanged content must stay idempotent, and the
/// unique index added in migration v11 would otherwise reject the insert.
/// Concerns attached to the replaced row go with it, so a re-run cannot leave
/// duplicate recommendations behind.
pub fn save_result(
    conn: &Connection,
    project_id: Uuid,
    dimension: &str,
    tier: &str,
    input_hash: &str,
    score: Option<f32>,
    data: &str,
) -> Result<i64> {
    let pid = project_id.to_string();
    for sql in [
        "DELETE FROM concerns WHERE result_id IN \
         (SELECT result_id FROM analysis_results WHERE project_id = ? AND dimension = ? AND input_hash = ?)",
        "DELETE FROM analysis_results WHERE project_id = ? AND dimension = ? AND input_hash = ?",
    ] {
        conn.execute(sql, params![pid, dimension, input_hash])
            .map_err(|e| crate::StudioError::Database(e.to_string()))?;
    }

    let result_id: i64 = conn
        .query_row("SELECT nextval('seq_analysis_results')", [], |row| {
            row.get(0)
        })
        .map_err(|e| crate::StudioError::Database(e.to_string()))?;

    conn.execute(
        "INSERT INTO analysis_results \
         (result_id, project_id, dimension, tier, input_hash, score, data, computed_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            result_id,
            pid,
            dimension,
            tier,
            input_hash,
            score,
            data,
            chrono::Utc::now().to_rfc3339()
        ],
    )
    .map_err(|e| crate::StudioError::Database(e.to_string()))?;

    Ok(result_id)
}

/// The stored row for `(project, dimension, input_hash)`, if one exists.
pub fn load_result(
    conn: &Connection,
    project_id: Uuid,
    dimension: &str,
    input_hash: &str,
) -> Result<Option<StoredResult>> {
    conn.query_row(
        "SELECT result_id, dimension, tier, input_hash, score, data \
         FROM analysis_results WHERE project_id = ? AND dimension = ? AND input_hash = ?",
        params![project_id.to_string(), dimension, input_hash],
        |row| {
            Ok(StoredResult {
                result_id: row.get(0)?,
                dimension: row.get(1)?,
                // Pre-v11 rows have no tier; they were all produced by T0.
                tier: row
                    .get::<_, Option<String>>(2)?
                    .unwrap_or_else(|| "t0".to_string()),
                input_hash: row.get(3)?,
                score: row.get(4)?,
                data: row.get(5)?,
            })
        },
    )
    .optional()
    .map_err(|e| crate::StudioError::Database(e.to_string()))
}

/// The stored output of `dimension`, deserialized back into the analysis type
/// that produced it (`T0Stats`, `T1Stats`, `Assessment`, …).
///
/// A row written by an older, incompatible version of that type surfaces as an
/// error rather than silently wrong data; callers treat that as a cache miss and
/// recompute.
pub fn load_cached<T: DeserializeOwned>(
    conn: &Connection,
    project_id: Uuid,
    dimension: &str,
    input_hash: &str,
) -> Result<Option<T>> {
    match load_result(conn, project_id, dimension, input_hash)? {
        None => Ok(None),
        Some(row) => serde_json::from_str(&row.data)
            .map(Some)
            .map_err(|e| crate::StudioError::Database(format!("Corrupt cached {dimension}: {e}"))),
    }
}

/// Drop every result stored for a project — the explicit "重新分析" path —
/// together with the concerns hanging off them. Returns the number of result
/// rows removed.
pub fn invalidate_project(conn: &Connection, project_id: Uuid) -> Result<u64> {
    conn.execute(
        "DELETE FROM concerns WHERE result_id IN \
         (SELECT result_id FROM analysis_results WHERE project_id = ?)",
        [project_id.to_string()],
    )
    .map_err(|e| crate::StudioError::Database(e.to_string()))?;
    let removed = conn
        .execute(
            "DELETE FROM analysis_results WHERE project_id = ?",
            [project_id.to_string()],
        )
        .map_err(|e| crate::StudioError::Database(e.to_string()))?;
    Ok(removed as u64)
}

/// Attach concerns to a stored result. Returns the number of rows written.
///
/// The `location` columns stay NULL for rule-based findings, which are
/// project-level; block-scoped concerns fill them in.
pub fn save_concerns(conn: &Connection, result_id: i64, concerns: &[Concern]) -> Result<usize> {
    let mut written = 0usize;
    for c in concerns {
        let concern_id: i64 = conn
            .query_row("SELECT nextval('seq_concerns')", [], |row| row.get(0))
            .map_err(|e| crate::StudioError::Database(e.to_string()))?;
        conn.execute(
            "INSERT INTO concerns \
             (concern_id, result_id, severity, title, description, \
              block_id, section_path, span_start, span_end, suggestion) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                concern_id,
                result_id,
                severity_to_str(&c.severity),
                c.title,
                c.description,
                c.location.block_id,
                c.location.section_path,
                c.location.span_start.map(|v| v as i64),
                c.location.span_end.map(|v| v as i64),
                c.suggestion
            ],
        )
        .map_err(|e| crate::StudioError::Database(e.to_string()))?;
        written += 1;
    }
    Ok(written)
}

/// Every concern recorded for a project, most severe first.
pub fn list_concerns(conn: &Connection, project_id: Uuid) -> Result<Vec<Concern>> {
    let mut stmt = conn
        .prepare(
            "SELECT c.severity, c.title, c.description, c.block_id, c.section_path, \
                    c.span_start, c.span_end, c.suggestion \
             FROM concerns c \
             JOIN analysis_results a ON a.result_id = c.result_id \
             WHERE a.project_id = ? \
             ORDER BY CASE c.severity \
                        WHEN 'critical' THEN 0 WHEN 'high' THEN 1 \
                        WHEN 'medium' THEN 2 ELSE 3 END, c.concern_id",
        )
        .map_err(|e| crate::StudioError::Database(e.to_string()))?;

    let rows = stmt
        .query_map([project_id.to_string()], |row| {
            let severity: String = row.get(0)?;
            Ok(Concern {
                severity: severity_from_str(&severity),
                title: row.get(1)?,
                description: row.get(2)?,
                location: ConcernLocation {
                    block_id: row.get(3)?,
                    section_path: row.get(4)?,
                    span_start: row.get::<_, Option<i64>>(5)?.map(|v| v as usize),
                    span_end: row.get::<_, Option<i64>>(6)?.map(|v| v as usize),
                },
                suggestion: row.get(7)?,
            })
        })
        .map_err(|e| crate::StudioError::Database(e.to_string()))?;

    let mut concerns = Vec::new();
    for row in rows {
        concerns.push(row.map_err(|e| crate::StudioError::Database(e.to_string()))?);
    }
    Ok(concerns)
}

fn severity_to_str(severity: &Severity) -> &'static str {
    match severity {
        Severity::Low => "low",
        Severity::Medium => "medium",
        Severity::High => "high",
        Severity::Critical => "critical",
    }
}

/// Unknown or legacy spellings fall back to `Medium` rather than being dropped:
/// losing a recommendation is worse than mis-rating it one level.
fn severity_from_str(severity: &str) -> Severity {
    match severity {
        "low" => Severity::Low,
        "high" => Severity::High,
        "critical" => Severity::Critical,
        _ => Severity::Medium,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::run_migrations(&conn).unwrap();
        conn
    }

    fn count(conn: &Connection, table: &str) -> i64 {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .unwrap()
    }

    fn concern(severity: Severity, title: &str) -> Concern {
        Concern {
            severity,
            title: title.into(),
            description: format!("{title} 的说明"),
            location: ConcernLocation {
                block_id: None,
                section_path: None,
                span_start: None,
                span_end: None,
            },
            suggestion: Some("建议".into()),
        }
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let conn = test_conn();
        let project = Uuid::new_v4();
        let id = save_result(
            &conn,
            project,
            "t0",
            "t0",
            "hash-1",
            None,
            "{\"word_count\":7}",
        )
        .unwrap();

        let row = load_result(&conn, project, "t0", "hash-1")
            .unwrap()
            .unwrap();
        assert_eq!(row.result_id, id);
        assert_eq!(row.tier, "t0");
        assert_eq!(row.data, "{\"word_count\":7}");

        // A different dimension or a different input is a miss, not a hit.
        assert!(load_result(&conn, project, "t1", "hash-1")
            .unwrap()
            .is_none());
        assert!(load_result(&conn, project, "t0", "hash-2")
            .unwrap()
            .is_none());
    }

    #[test]
    fn test_load_cached_deserializes() {
        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
        struct Payload {
            word_count: u64,
            top: Vec<(String, u32)>,
        }

        let conn = test_conn();
        let project = Uuid::new_v4();
        let payload = Payload {
            word_count: 3,
            top: vec![("甲".into(), 2), ("乙".into(), 1)],
        };
        save_result(
            &conn,
            project,
            "t0",
            "t0",
            "h",
            None,
            &serde_json::to_string(&payload).unwrap(),
        )
        .unwrap();

        let hit = load_cached::<Payload>(&conn, project, "t0", "h").unwrap();
        assert_eq!(hit, Some(payload));
        assert!(load_cached::<Payload>(&conn, project, "t0", "other")
            .unwrap()
            .is_none());
    }

    #[test]
    fn test_save_result_is_idempotent_per_key() {
        let conn = test_conn();
        let project = Uuid::new_v4();
        save_result(&conn, project, "t0", "t0", "h", None, "{\"v\":1}").unwrap();
        save_result(&conn, project, "t0", "t0", "h", None, "{\"v\":2}").unwrap();

        assert_eq!(count(&conn, "analysis_results"), 1);
        let row = load_result(&conn, project, "t0", "h").unwrap().unwrap();
        assert_eq!(row.data, "{\"v\":2}", "the newer result must win");
    }

    #[test]
    fn test_invalidate_project_clears_results_and_concerns() {
        let conn = test_conn();
        let project = Uuid::new_v4();
        let other = Uuid::new_v4();

        let id = save_result(&conn, project, "assessment", "t1", "h", Some(72.5), "{}").unwrap();
        save_concerns(
            &conn,
            id,
            &[
                concern(Severity::High, "可读性偏低"),
                concern(Severity::Medium, "章节失衡"),
            ],
        )
        .unwrap();
        let other_id = save_result(&conn, other, "t0", "t0", "h", None, "{}").unwrap();
        save_concerns(&conn, other_id, &[concern(Severity::Low, "无关项目")]).unwrap();

        assert_eq!(invalidate_project(&conn, project).unwrap(), 1);

        assert_eq!(count(&conn, "analysis_results"), 1);
        assert_eq!(count(&conn, "concerns"), 1);
        assert!(load_result(&conn, other, "t0", "h").unwrap().is_some());
    }

    #[test]
    fn test_concerns_roundtrip_most_severe_first() {
        let conn = test_conn();
        let project = Uuid::new_v4();
        let id = save_result(&conn, project, "assessment", "t1", "h", None, "{}").unwrap();
        save_concerns(
            &conn,
            id,
            &[
                concern(Severity::Medium, "中"),
                concern(Severity::High, "高"),
                concern(Severity::Low, "低"),
            ],
        )
        .unwrap();

        let listed = list_concerns(&conn, project).unwrap();
        let severities: Vec<Severity> = listed.iter().map(|c| c.severity.clone()).collect();
        assert_eq!(
            severities,
            vec![Severity::High, Severity::Medium, Severity::Low]
        );
        assert_eq!(listed[0].description, "高 的说明");
        assert_eq!(listed[0].suggestion.as_deref(), Some("建议"));
        assert!(list_concerns(&conn, Uuid::new_v4()).unwrap().is_empty());
    }

    #[test]
    fn test_unknown_severity_falls_back_to_medium() {
        assert_eq!(severity_from_str("blocker"), Severity::Medium);
        assert_eq!(severity_to_str(&Severity::Critical), "critical");
    }
}
