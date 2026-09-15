//! Document repository — save and load document data.

use duckdb::Connection;
use studio_core::document::{DocumentData, SemanticBlock, Token};
use studio_core::entity::Entity;
use studio_core::noun_signal::NounSignal;
use uuid::Uuid;

use crate::Result;

/// Save a complete document with all its blocks, tokens, entities, and noun signals.
pub fn save_document(conn: &Connection, project_id: Uuid, doc: &DocumentData) -> Result<()> {
    conn.execute(
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

    // 全局递增 ID，避免跨 block 主键冲突
    let mut block_id_counter: i64 = 0;
    let mut token_id_counter: i64 = 0;
    let mut entity_id_counter: i64 = 0;
    let mut signal_id_counter: i64 = 0;

    for block in doc.blocks.iter() {
        block_id_counter += 1;
        let block_id = block_id_counter;
        conn.execute(
            "INSERT INTO semantic_blocks (block_id, doc_id, content, section_path, block_type, title)
             VALUES (?, ?, ?, ?, ?, ?)",
            duckdb::params![
                block_id, doc.doc_id.to_string(), block.content,
                block.section_path, block.block_type, block.title,
            ],
        )
        .map_err(|e| crate::StudioError::Database(e.to_string()))?;

        for token in block.tokens.iter() {
            token_id_counter += 1;
            conn.execute(
                "INSERT INTO tokens (token_id, block_id, text, pos, confidence, span_start, span_end, source)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                duckdb::params![
                    token_id_counter, block_id, token.text, token.pos,
                    token.confidence as f64, token.span.0 as i64, token.span.1 as i64, token.source,
                ],
            )
            .map_err(|e| crate::StudioError::Database(e.to_string()))?;
        }

        for entity in block.entities.iter() {
            entity_id_counter += 1;
            conn.execute(
                "INSERT INTO entities (entity_id, block_id, text, category, confidence, source, keep, filter, filter_reason, span_start, span_end)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                duckdb::params![
                    entity_id_counter, block_id, entity.text, entity.category.as_str(),
                    entity.confidence as f64, entity.source, entity.keep,
                    entity.filter, entity.filter_reason,
                    entity.span.0 as i64, entity.span.1 as i64,
                ],
            )
            .map_err(|e| crate::StudioError::Database(e.to_string()))?;
        }

        for signal in block.noun_signals.iter() {
            signal_id_counter += 1;
            conn.execute(
                "INSERT INTO noun_signals (signal_id, block_id, text, pos, syntactic_role, score, span_start, span_end)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                duckdb::params![
                    signal_id_counter, block_id, signal.text, signal.pos,
                    signal.syntactic_role, signal.score as f64,
                    signal.span.0 as i64, signal.span.1 as i64,
                ],
            )
            .map_err(|e| crate::StudioError::Database(e.to_string()))?;
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
            "SELECT block_id, content, section_path, block_type, title
             FROM semantic_blocks WHERE doc_id = ? ORDER BY block_id",
        )
        .map_err(|e| crate::StudioError::Database(e.to_string()))?;

    let block_rows = stmt
        .query_map([doc_id.to_string()], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })
        .map_err(|e| crate::StudioError::Database(e.to_string()))?;

    let mut blocks = Vec::new();
    for row in block_rows {
        let (block_id, content, section_path, block_type, title) =
            row.map_err(|e| crate::StudioError::Database(e.to_string()))?;
        let tokens = load_tokens(conn, block_id)?;
        let entities = load_entities(conn, block_id)?;
        let noun_signals = load_noun_signals(conn, block_id)?;
        blocks.push(SemanticBlock {
            block_ids: vec![block_id as u32],
            content,
            section_path,
            block_type,
            title,
            tokens,
            entities,
            noun_signals,
        });
    }

    Ok(DocumentData {
        doc_id,
        title,
        blocks,
        total_word_count: word_count as u64,
        total_char_count: char_count as u64,
    })
}

fn load_tokens(conn: &Connection, block_id: i64) -> Result<Vec<Token>> {
    let mut stmt = conn
        .prepare(
            "SELECT text, pos, confidence, span_start, span_end, source
             FROM tokens WHERE block_id = ? ORDER BY token_id",
        )
        .map_err(|e| crate::StudioError::Database(e.to_string()))?;
    let rows = stmt
        .query_map([block_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, f64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, String>(5)?,
            ))
        })
        .map_err(|e| crate::StudioError::Database(e.to_string()))?;
    let tokens: Vec<Token> = rows
        .filter_map(|r| r.ok().map(|(text, pos, conf, s0, s1, source)| Token {
            text, pos, confidence: conf as f32, span: (s0 as usize, s1 as usize), source,
        }))
        .collect();
    Ok(tokens)
}

fn load_entities(conn: &Connection, block_id: i64) -> Result<Vec<Entity>> {
    let mut stmt = conn
        .prepare(
            "SELECT text, category, confidence, source, keep, filter, filter_reason, span_start, span_end
             FROM entities WHERE block_id = ? ORDER BY entity_id",
        )
        .map_err(|e| crate::StudioError::Database(e.to_string()))?;
    let rows = stmt
        .query_map([block_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, f64>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, bool>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, i64>(7)?,
                row.get::<_, i64>(8)?,
            ))
        })
        .map_err(|e| crate::StudioError::Database(e.to_string()))?;
    let entities: Vec<Entity> = rows
        .filter_map(|r| {
            r.ok().map(|(text, cat, conf, source, keep, filter, filter_reason, s0, s1)| Entity {
                text, category: cat.parse().unwrap_or_default(),
                confidence: conf as f32, source, keep, filter, filter_reason,
                span: (s0 as usize, s1 as usize),
            })
        })
        .collect();
    Ok(entities)
}

fn load_noun_signals(conn: &Connection, block_id: i64) -> Result<Vec<NounSignal>> {
    let mut stmt = conn
        .prepare(
            "SELECT text, pos, syntactic_role, score, span_start, span_end
             FROM noun_signals WHERE block_id = ? ORDER BY signal_id",
        )
        .map_err(|e| crate::StudioError::Database(e.to_string()))?;
    let rows = stmt
        .query_map([block_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, f64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, i64>(5)?,
            ))
        })
        .map_err(|e| crate::StudioError::Database(e.to_string()))?;
    let noun_signals: Vec<NounSignal> = rows
        .filter_map(|r| {
            r.ok().map(|(text, pos, role, score, s0, s1)| NounSignal {
                text, pos, syntactic_role: role, score: score as f32,
                span: (s0 as usize, s1 as usize), evidence: None,
            })
        })
        .collect();
    Ok(noun_signals)
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
                block_ids: vec![1],
                content: "Hello world".into(),
                section_path: "Ch1".into(),
                block_type: "paragraph".into(),
                title: None,
                tokens: vec![Token {
                    text: "Hello".into(), pos: "NNP".into(),
                    confidence: 0.99, span: (0, 5), source: "test".into(),
                }],
                entities: vec![Entity {
                    text: "Hello".into(), category: EntityCategory::Person,
                    confidence: 0.9, source: "test".into(),
                    keep: true, filter: None, filter_reason: None, span: (0, 5),
                }],
                noun_signals: vec![NounSignal {
                    text: "Hello".into(), pos: "NNP".into(),
                    syntactic_role: Some("Subject".into()),
                    score: 0.8, span: (0, 5), evidence: None,
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
        save_document(&conn, project_id, &doc).unwrap();
        let loaded = load_document(&conn, doc.doc_id).unwrap();
        assert_eq!(loaded.title, "Test Doc");
        assert_eq!(loaded.blocks.len(), 1);
        assert_eq!(loaded.blocks[0].content, "Hello world");
        assert_eq!(loaded.blocks[0].tokens.len(), 1);
        assert_eq!(loaded.blocks[0].tokens[0].text, "Hello");
        assert_eq!(loaded.blocks[0].entities.len(), 1);
        assert_eq!(loaded.blocks[0].entities[0].category, EntityCategory::Person);
        assert_eq!(loaded.blocks[0].noun_signals.len(), 1);
    }

    #[test]
    fn test_load_nonexistent() {
        let conn = test_conn();
        let result = load_document(&conn, Uuid::new_v4());
        assert!(result.is_err());
    }
}