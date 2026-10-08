//! Document repository — save and load document data.

use std::collections::HashMap;

use duckdb::{Connection, Transaction};
use studio_core::document::{DocumentData, SemanticBlock, Token};
use studio_core::entity::Entity;
use studio_core::noun_signal::NounSignal;
use uuid::Uuid;

use crate::Result;

/// Maximum number of `on_progress` invocations per `save_document` call.
///
/// Progress is reported at most once per `total / 100` blocks (plus the final
/// update), so a 10k-block document emits ~100 callbacks instead of 10k. This
/// keeps the Tauri event stream smooth without flooding the frontend.
const PROGRESS_REPORT_INTERVAL: u64 = 100;

/// Save a complete document with all its blocks, tokens, entities, and noun signals.
///
/// Uses DuckDB's [Appender](duckdb::Appender) bulk-insert API inside a single
/// transaction: rows are buffered in memory and flushed in large batches, which
/// is orders of magnitude faster than per-row `INSERT` statements (the old
/// approach prepared and auto-committed a statement for every row). The
/// transaction makes the import atomic — if anything fails, the whole document
/// is rolled back and no partial data is left behind.
///
/// `on_progress` is invoked (throttled to at most ~100 calls) with
/// (`current`, `total`) block counts as blocks are written, so callers can
/// report live progress. Pass `None` to skip it.
pub fn save_document(
    conn: &Connection,
    project_id: Uuid,
    doc: &DocumentData,
    on_progress: Option<&dyn Fn(u64, u64)>,
) -> Result<()> {
    // One transaction for the whole document. Dropping `tx` without commit
    // (i.e. on any error below) rolls everything back.
    let tx = Transaction::new_unchecked(conn)
        .map_err(|e| crate::StudioError::Database(e.to_string()))?;

    tx.execute(
        "INSERT INTO documents (doc_id, project_id, title, total_word_count, total_char_count)
         VALUES (?, ?, ?, ?, ?)",
        duckdb::params![
            doc.doc_id.to_string(),
            project_id.to_string(),
            doc.title,
            doc.total_word_count as i64,
            doc.total_char_count as i64,
        ],
    )
    .map_err(|e| crate::StudioError::Database(e.to_string()))?;

    // One appender per child table, all active for the duration of the block
    // loop. Append order matches each table's column order.
    let mut blocks_app = tx
        .appender("semantic_blocks")
        .map_err(|e| crate::StudioError::Database(e.to_string()))?;
    let mut tokens_app = tx
        .appender("tokens")
        .map_err(|e| crate::StudioError::Database(e.to_string()))?;
    let mut entities_app = tx
        .appender("entities")
        .map_err(|e| crate::StudioError::Database(e.to_string()))?;
    let mut signals_app = tx
        .appender("noun_signals")
        .map_err(|e| crate::StudioError::Database(e.to_string()))?;

    let doc_id = doc.doc_id.to_string();
    let total_blocks = doc.blocks.len() as u64;
    let report_every = (total_blocks / PROGRESS_REPORT_INTERVAL).max(1);
    let mut last_reported: u64 = 0;

    for (seq, block) in doc.blocks.iter().enumerate() {
        let current = seq as u64;
        if let Some(cb) = on_progress {
            if current.is_multiple_of(report_every) || current + 1 == total_blocks {
                cb(current, total_blocks);
                last_reported = current;
            }
        }
        // Use the TraceView source UUID as the block's globally-unique key.
        // Fall back to a generated UUID if the source did not provide one, so
        // the primary key is always unique.
        let source_block_id = if block.source_block_id.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            block.source_block_id.clone()
        };
        blocks_app
            .append_row(duckdb::params![
                source_block_id,
                doc_id,
                seq as i64,
                block.content,
                block.section_path,
                block.block_type,
                block.title,
            ])
            .map_err(|e| crate::StudioError::Database(e.to_string()))?;

        for (tseq, token) in block.tokens.iter().enumerate() {
            tokens_app
                .append_row(duckdb::params![
                    Uuid::new_v4().to_string(),
                    source_block_id,
                    tseq as i64,
                    token.text,
                    token.pos,
                    token.confidence as f64,
                    token.span.0 as i64,
                    token.span.1 as i64,
                    token.source,
                ])
                .map_err(|e| crate::StudioError::Database(e.to_string()))?;
        }

        for (eseq, entity) in block.entities.iter().enumerate() {
            entities_app
                .append_row(duckdb::params![
                    Uuid::new_v4().to_string(),
                    source_block_id,
                    eseq as i64,
                    entity.text,
                    entity.category.as_str(),
                    entity.confidence as f64,
                    entity.source,
                    entity.keep,
                    entity.filter,
                    entity.filter_reason,
                    entity.span.0 as i64,
                    entity.span.1 as i64,
                ])
                .map_err(|e| crate::StudioError::Database(e.to_string()))?;
        }

        for (sseq, signal) in block.noun_signals.iter().enumerate() {
            signals_app
                .append_row(duckdb::params![
                    Uuid::new_v4().to_string(),
                    source_block_id,
                    sseq as i64,
                    signal.text,
                    signal.pos,
                    signal.syntactic_role,
                    signal.score as f64,
                    signal.span.0 as i64,
                    signal.span.1 as i64,
                ])
                .map_err(|e| crate::StudioError::Database(e.to_string()))?;
        }
    }

    // Flush explicitly and check the result: constraint violations surface at
    // flush time, and the implicit flush on drop would discard the error.
    blocks_app
        .flush()
        .map_err(|e| crate::StudioError::Database(e.to_string()))?;
    tokens_app
        .flush()
        .map_err(|e| crate::StudioError::Database(e.to_string()))?;
    entities_app
        .flush()
        .map_err(|e| crate::StudioError::Database(e.to_string()))?;
    signals_app
        .flush()
        .map_err(|e| crate::StudioError::Database(e.to_string()))?;

    // Drop the appenders (releasing their borrow of `tx`) before committing.
    drop((blocks_app, tokens_app, entities_app, signals_app));

    tx.commit()
        .map_err(|e| crate::StudioError::Database(e.to_string()))?;

    // Final progress update so the UI reaches 100%.
    if let Some(cb) = on_progress {
        if total_blocks > 0 && last_reported + 1 != total_blocks {
            cb(total_blocks - 1, total_blocks);
        }
    }
    Ok(())
}

/// Load a complete document by ID.
pub fn load_document(conn: &Connection, doc_id: Uuid) -> Result<DocumentData> {
    let (title, word_count, char_count): (String, i64, i64) = conn
        .query_row(
            "SELECT title, total_word_count, total_char_count FROM documents WHERE doc_id = ?",
            [doc_id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|e| crate::StudioError::Database(format!("Document not found: {e}")))?;

    let mut stmt = conn
        .prepare(
            "SELECT source_block_id, content, section_path, block_type, title
             FROM semantic_blocks WHERE doc_id = ? ORDER BY seq",
        )
        .map_err(|e| crate::StudioError::Database(e.to_string()))?;

    let block_rows = stmt
        .query_map([doc_id.to_string()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })
        .map_err(|e| crate::StudioError::Database(e.to_string()))?;

    let mut blocks: Vec<SemanticBlock> = Vec::new();
    for row in block_rows {
        let (source_block_id, content, section_path, block_type, title) =
            row.map_err(|e| crate::StudioError::Database(e.to_string()))?;
        blocks.push(SemanticBlock {
            source_block_id,
            content,
            section_path,
            block_type,
            title,
            tokens: vec![],
            entities: vec![],
            noun_signals: vec![],
        });
    }

    if blocks.is_empty() {
        return Ok(DocumentData {
            doc_id,
            title,
            blocks,
            total_word_count: word_count as u64,
            total_char_count: char_count as u64,
        });
    }

    // Batch-load the child rows (3 queries total, scoped to this document) and
    // attach them to their blocks by source_block_id. This replaces the previous
    // per-block N+1 pattern (3 queries per block) with a constant number of
    // queries, which keeps analysis fast on large documents.
    let mut tokens_by_block = load_tokens_for_doc(conn, doc_id)?;
    let mut entities_by_block = load_entities_for_doc(conn, doc_id)?;
    let mut signals_by_block = load_signals_for_doc(conn, doc_id)?;

    for block in blocks.iter_mut() {
        let key = block.source_block_id.clone();
        block.tokens = tokens_by_block.remove(&key).unwrap_or_default();
        block.entities = entities_by_block.remove(&key).unwrap_or_default();
        block.noun_signals = signals_by_block.remove(&key).unwrap_or_default();
    }

    Ok(DocumentData {
        doc_id,
        title,
        blocks,
        total_word_count: word_count as u64,
        total_char_count: char_count as u64,
    })
}

/// Load all tokens belonging to a document's blocks, grouped by source_block_id.
///
/// A single query scoped to the document (via a subquery on `semantic_blocks`)
/// replaces the previous one-query-per-block pattern.
fn load_tokens_for_doc(conn: &Connection, doc_id: Uuid) -> Result<HashMap<String, Vec<Token>>> {
    let mut stmt = conn
        .prepare(
            "SELECT source_block_id, text, pos, confidence, span_start, span_end, source
             FROM tokens
             WHERE source_block_id IN (SELECT source_block_id FROM semantic_blocks WHERE doc_id = ?)
             ORDER BY source_block_id, seq",
        )
        .map_err(|e| crate::StudioError::Database(e.to_string()))?;
    let rows = stmt
        .query_map([doc_id.to_string()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, f64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, String>(6)?,
            ))
        })
        .map_err(|e| crate::StudioError::Database(e.to_string()))?;
    let mut by_block: HashMap<String, Vec<Token>> = HashMap::new();
    for row in rows {
        let (source_block_id, text, pos, conf, s0, s1, source) =
            row.map_err(|e| crate::StudioError::Database(e.to_string()))?;
        by_block.entry(source_block_id).or_default().push(Token {
            text,
            pos,
            confidence: conf as f32,
            span: (s0 as usize, s1 as usize),
            source,
        });
    }
    Ok(by_block)
}

/// Load all entities belonging to a document's blocks, grouped by source_block_id.
fn load_entities_for_doc(conn: &Connection, doc_id: Uuid) -> Result<HashMap<String, Vec<Entity>>> {
    let mut stmt = conn
        .prepare(
            "SELECT source_block_id, text, category, confidence, source, keep, filter, filter_reason, span_start, span_end
             FROM entities
             WHERE source_block_id IN (SELECT source_block_id FROM semantic_blocks WHERE doc_id = ?)
             ORDER BY source_block_id, seq",
        )
        .map_err(|e| crate::StudioError::Database(e.to_string()))?;
    let rows = stmt
        .query_map([doc_id.to_string()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, f64>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, bool>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, i64>(8)?,
                row.get::<_, i64>(9)?,
            ))
        })
        .map_err(|e| crate::StudioError::Database(e.to_string()))?;
    let mut by_block: HashMap<String, Vec<Entity>> = HashMap::new();
    for row in rows {
        let (source_block_id, text, cat, conf, source, keep, filter, filter_reason, s0, s1) =
            row.map_err(|e| crate::StudioError::Database(e.to_string()))?;
        by_block.entry(source_block_id).or_default().push(Entity {
            text,
            category: cat.parse().unwrap_or_default(),
            confidence: conf as f32,
            source,
            keep,
            filter,
            filter_reason,
            span: (s0 as usize, s1 as usize),
        });
    }
    Ok(by_block)
}

/// Load all noun signals belonging to a document's blocks, grouped by source_block_id.
fn load_signals_for_doc(
    conn: &Connection,
    doc_id: Uuid,
) -> Result<HashMap<String, Vec<NounSignal>>> {
    let mut stmt = conn
        .prepare(
            "SELECT source_block_id, text, pos, syntactic_role, score, span_start, span_end
             FROM noun_signals
             WHERE source_block_id IN (SELECT source_block_id FROM semantic_blocks WHERE doc_id = ?)
             ORDER BY source_block_id, seq",
        )
        .map_err(|e| crate::StudioError::Database(e.to_string()))?;
    let rows = stmt
        .query_map([doc_id.to_string()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, f64>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
            ))
        })
        .map_err(|e| crate::StudioError::Database(e.to_string()))?;
    let mut by_block: HashMap<String, Vec<NounSignal>> = HashMap::new();
    for row in rows {
        let (source_block_id, text, pos, role, score, s0, s1) =
            row.map_err(|e| crate::StudioError::Database(e.to_string()))?;
        by_block
            .entry(source_block_id)
            .or_default()
            .push(NounSignal {
                text,
                pos,
                syntactic_role: role,
                score: score as f32,
                span: (s0 as usize, s1 as usize),
                evidence: None,
            });
    }
    Ok(by_block)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::migration::run_migrations;
    use studio_core::entity::EntityCategory;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    fn sample_doc() -> DocumentData {
        DocumentData {
            doc_id: Uuid::new_v4(),
            title: "Test Doc".into(),
            blocks: vec![SemanticBlock {
                source_block_id: "blk-1".into(),
                content: "Hello world".into(),
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
            total_char_count: 11,
        }
    }

    #[test]
    fn test_save_and_load_document() {
        let conn = test_conn();
        let project_id = Uuid::new_v4();
        let doc = sample_doc();
        save_document(&conn, project_id, &doc, None).unwrap();
        let loaded = load_document(&conn, doc.doc_id).unwrap();
        assert_eq!(loaded.title, "Test Doc");
        assert_eq!(loaded.blocks.len(), 1);
        assert_eq!(loaded.blocks[0].content, "Hello world");
        assert_eq!(loaded.blocks[0].tokens.len(), 1);
        assert_eq!(loaded.blocks[0].tokens[0].text, "Hello");
        assert_eq!(loaded.blocks[0].entities.len(), 1);
        assert_eq!(
            loaded.blocks[0].entities[0].category,
            EntityCategory::Person
        );
        assert_eq!(loaded.blocks[0].noun_signals.len(), 1);
    }

    #[test]
    fn test_load_nonexistent() {
        let conn = test_conn();
        let result = load_document(&conn, Uuid::new_v4());
        assert!(result.is_err());
    }

    fn block_with_id(id: &str) -> SemanticBlock {
        SemanticBlock {
            source_block_id: id.into(),
            content: "content".into(),
            section_path: "Ch1".into(),
            block_type: "paragraph".into(),
            title: None,
            tokens: vec![],
            entities: vec![],
            noun_signals: vec![],
        }
    }

    #[test]
    fn test_save_document_rolls_back_on_failure() {
        let conn = test_conn();
        let project_id = Uuid::new_v4();
        // Two blocks sharing the same source_block_id -> primary-key
        // collision, which the appender reports at flush time.
        let doc = DocumentData {
            doc_id: Uuid::new_v4(),
            title: "Dup".into(),
            blocks: vec![block_with_id("dup-1"), block_with_id("dup-1")],
            total_word_count: 2,
            total_char_count: 14,
        };
        assert!(save_document(&conn, project_id, &doc, None).is_err());
        // The whole document must be rolled back: no partial rows anywhere.
        for table in [
            "documents",
            "semantic_blocks",
            "tokens",
            "entities",
            "noun_signals",
        ] {
            let n: i64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
                .unwrap();
            assert_eq!(n, 0, "expected {table} to be empty after rollback");
        }
    }

    #[test]
    fn test_save_document_progress_throttled() {
        let conn = test_conn();
        let project_id = Uuid::new_v4();
        let doc = DocumentData {
            doc_id: Uuid::new_v4(),
            title: "Big".into(),
            blocks: (0..2000)
                .map(|i| block_with_id(&format!("blk-{i}")))
                .collect(),
            total_word_count: 0,
            total_char_count: 0,
        };
        // `Cell` gives interior mutability, so the closure can mutate it without
        // the binding itself being `mut`.
        let calls = std::cell::Cell::new(0u64);
        save_document(
            &conn,
            project_id,
            &doc,
            Some(&|_current, _total| {
                calls.set(calls.get() + 1);
            }),
        )
        .unwrap();
        // Throttled to ~100 updates + final, not one call per block.
        assert!(
            calls.get() <= 110,
            "expected throttled progress, got {} calls",
            calls.get()
        );
        assert!(
            calls.get() >= 2,
            "expected start and end updates, got {}",
            calls.get()
        );
    }
}
