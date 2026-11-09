//! Entity registry — stable, content-derived identity for entities.
//!
//! The per-import `entity_id` UUID is a row key, not an identity: the same
//! person in two imports gets two rows and two ids. The registry issues a
//! deterministic `stable_id` per (normalized text, category) so identity
//! survives re-imports, projects, and exports — the precondition for
//! NarratSphere to merge a fresh export into a hand-edited knowledge sphere
//! by node id instead of overwriting it.

use duckdb::{Connection, Transaction};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use studio_core::entity::EntityStat;
use uuid::Uuid;

use crate::Result;

/// Normalize text for identity: trim, lowercase, fullwidth ASCII → halfwidth,
/// collapse internal whitespace. Conservative on purpose — alias resolution
/// ("马洛" vs "马洛先生") is a deliberate merge, not a silent normalization.
pub fn normalize_key(text: &str) -> String {
    let folded: String = text
        .trim()
        .chars()
        .map(|c| {
            // Fullwidth ASCII block (FF01–FF5E) → its ASCII counterpart.
            let code = c as u32;
            if (0xFF01..=0xFF5E).contains(&code) {
                char::from_u32(code - 0xFEE0).unwrap_or(c)
            } else {
                c
            }
        })
        .collect();
    folded
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Content-derived stable id for one identity: `ent:<16 hex>` of the
/// SHA-256 over `category ⿟ normalized_text`. Pure and deterministic — the
/// same identity hashes to the same id in every project and every run, which
/// is also how rows written before v13 (NULL `stable_id`) are backfilled on
/// read.
pub fn stable_id_for(text: &str, category: &str) -> String {
    let key = normalize_key(text);
    let mut hasher = Sha256::new();
    hasher.update(category.as_bytes());
    hasher.update([0x1F]);
    hasher.update(key.as_bytes());
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(16);
    for byte in &digest[..8] {
        hex.push_str(&format!("{byte:02x}"));
    }
    format!("ent:{hex}")
}

/// Resolve the stable id for one entity, registering it on first sight.
pub fn resolve_or_insert(tx: &Transaction<'_>, text: &str, category: &str) -> Result<String> {
    let key = normalize_key(text);
    let existing: Option<String> = tx
        .query_row(
            "SELECT stable_id FROM entity_registry WHERE canonical_text = ? AND category = ?",
            duckdb::params![key, category],
            |row| row.get(0),
        )
        .ok();
    if let Some(id) = existing {
        return Ok(id);
    }

    let id = stable_id_for(text, category);
    tx.execute(
        "INSERT INTO entity_registry (stable_id, canonical_text, category)
         VALUES (?, ?, ?)
         ON CONFLICT (stable_id) DO NOTHING",
        duckdb::params![id, key, category],
    )
    .map_err(|e| crate::StudioError::Database(e.to_string()))?;
    Ok(id)
}

/// One row per entity identity in a project: the surface-layer raw material
/// for the knowsphere export. The population matches the analysis layer's
/// (every extracted entity counts — the analysis dimensions never consult
/// `keep`/`filter` either), so the export always matches what the analysis
/// page shows. Rows without a registry id get their id computed from
/// (text, category) — the same value the registry would have issued — and
/// merged into the row for that identity.
pub fn entity_stats_for_project(conn: &Connection, project_id: Uuid) -> Result<Vec<EntityStat>> {
    let mut stmt = conn
        .prepare(
            "SELECT e.stable_id, e.text, e.category,
                    COUNT(*) AS mentions, MAX(e.confidence) AS conf
             FROM entities e
             JOIN semantic_blocks b ON e.source_block_id = b.source_block_id
             JOIN documents d ON b.doc_id = d.doc_id
             WHERE d.project_id = ?
             GROUP BY e.stable_id, e.text, e.category
             ORDER BY mentions DESC, e.text ASC",
        )
        .map_err(|e| crate::StudioError::Database(e.to_string()))?;
    let rows = stmt
        .query_map([project_id.to_string()], |row| {
            Ok((
                row.get::<_, Option<String>>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)? as u64,
                row.get::<_, f64>(4)?,
            ))
        })
        .map_err(|e| crate::StudioError::Database(e.to_string()))?;

    // Insertion order is "busiest first", so the first row for an identity
    // contributes the label spelling and the rest only add up.
    let mut by_id: HashMap<String, EntityStat> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    for row in rows {
        let (stored_id, text, category, mentions, conf) =
            row.map_err(|e| crate::StudioError::Database(e.to_string()))?;
        let stable_id = stored_id.unwrap_or_else(|| stable_id_for(&text, &category));
        match by_id.get_mut(&stable_id) {
            Some(stat) => {
                stat.mentions += mentions;
                stat.confidence = stat.confidence.max(conf);
            }
            None => {
                order.push(stable_id.clone());
                by_id.insert(
                    stable_id.clone(),
                    EntityStat {
                        stable_id,
                        text,
                        category,
                        mentions,
                        confidence: conf,
                    },
                );
            }
        }
    }
    Ok(order
        .into_iter()
        .filter_map(|id| by_id.remove(&id))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use duckdb::Connection;

    fn conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::run_migrations(&conn).unwrap();
        conn
    }

    #[test]
    fn normalize_key_folds_case_width_and_space() {
        assert_eq!(normalize_key("  Ｍarlowe  ３.０ "), "marlowe 3.0");
        assert_eq!(normalize_key("马洛"), "马洛");
    }

    #[test]
    fn stable_id_is_deterministic_and_identity_scoped() {
        let a = stable_id_for("马洛", "PERSON");
        let b = stable_id_for(" 马洛 ", "PERSON");
        let c = stable_id_for("马洛", "LOCATION");
        assert_eq!(a, b, "normalization must not change the id");
        assert_ne!(a, c, "category is part of the identity");
        assert!(a.starts_with("ent:"));
        assert_eq!(a.len(), 4 + 16);
    }

    #[test]
    fn registry_reuses_ids_across_transactions() {
        let mut conn = conn();
        let id1 = {
            let tx = conn.transaction().unwrap();
            let id = resolve_or_insert(&tx, "Marlowe", "PERSON").unwrap();
            tx.commit().unwrap();
            id
        };
        let id2 = {
            let tx = conn.transaction().unwrap();
            let id = resolve_or_insert(&tx, "marlowe ", "PERSON").unwrap();
            tx.commit().unwrap();
            id
        };
        assert_eq!(id1, id2);
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM entity_registry", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn stats_aggregate_per_identity_and_backfill_null_ids() {
        let mut conn = conn();
        let project = Uuid::new_v4();
        let doc_id = Uuid::new_v4();
        conn.execute(
            "INSERT INTO documents (doc_id, project_id, title, total_word_count, total_char_count)
             VALUES (?, ?, 'd', 0, 0)",
            duckdb::params![doc_id.to_string(), project.to_string()],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO semantic_blocks (source_block_id, doc_id, seq, content, section_path, block_type, title)
             VALUES ('b1', ?, 0, 'x', 's', 'paragraph', NULL)",
            [doc_id.to_string()],
        )
        .unwrap();
        // One row with a registry id, one legacy row without, same identity.
        // The population follows the analysis layer, which never consults
        // keep/filter — a dropped-but-extracted entity still counts here.
        let id = {
            let tx = conn.transaction().unwrap();
            let id = resolve_or_insert(&tx, "马洛", "PERSON").unwrap();
            tx.commit().unwrap();
            id
        };
        conn.execute(
            "INSERT INTO entities VALUES (?, 'b1', 0, '马洛', 'PERSON', 0.9, 't', TRUE, NULL, NULL, 0, 1, ?)",
            duckdb::params![Uuid::new_v4().to_string(), id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO entities VALUES (?, 'b1', 1, '马洛', 'PERSON', 0.8, 't', TRUE, NULL, NULL, 2, 4, NULL)",
            [Uuid::new_v4().to_string()],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO entities VALUES (?, 'b1', 2, '噪声', 'UNKNOWN', 0.5, 't', FALSE, NULL, NULL, 5, 7, NULL)",
            [Uuid::new_v4().to_string()],
        )
        .unwrap();

        let stats = crate::entity_stats_for_project(&conn, project).unwrap();
        assert_eq!(stats.len(), 2);
        let marlowe = stats
            .iter()
            .find(|s| s.text == "马洛")
            .expect("marlowe row");
        assert_eq!(marlowe.stable_id, id);
        assert_eq!(marlowe.mentions, 2);
        // The column is FLOAT: compare with f32-derived tolerance.
        assert!((marlowe.confidence - 0.9).abs() < 1e-6);
    }
}
