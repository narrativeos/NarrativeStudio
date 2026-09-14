//! Database migration support.

use duckdb::Connection;

use crate::Result;

/// Run all pending migrations on the given connection.
pub fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS _migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        );",
    )
    .map_err(|e| crate::StudioError::Database(e.to_string()))?;

    migrate(
        conn,
        1,
        "create_projects",
        "
        CREATE TABLE IF NOT EXISTS projects (
            project_id UUID PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            genre TEXT,
            language TEXT NOT NULL DEFAULT 'zh-CN',
            created_at TIMESTAMP NOT NULL,
            updated_at TIMESTAMP NOT NULL
        );
    ",
    )?;

    migrate(
        conn,
        2,
        "create_documents",
        "
        CREATE TABLE IF NOT EXISTS documents (
            doc_id UUID PRIMARY KEY,
            project_id UUID NOT NULL,
            title TEXT NOT NULL,
            total_word_count BIGINT NOT NULL DEFAULT 0,
            total_char_count BIGINT NOT NULL DEFAULT 0,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
    ",
    )?;

    migrate(
        conn,
        3,
        "create_semantic_blocks",
        "
        CREATE TABLE IF NOT EXISTS semantic_blocks (
            block_id INTEGER PRIMARY KEY,
            doc_id UUID NOT NULL,
            content TEXT NOT NULL,
            section_path TEXT NOT NULL,
            block_type TEXT NOT NULL,
            title TEXT
        );
    ",
    )?;

    migrate(
        conn,
        4,
        "create_tokens",
        "
        CREATE TABLE IF NOT EXISTS tokens (
            token_id INTEGER PRIMARY KEY,
            block_id INTEGER NOT NULL,
            text TEXT NOT NULL,
            pos TEXT NOT NULL,
            confidence REAL NOT NULL,
            span_start INTEGER NOT NULL,
            span_end INTEGER NOT NULL,
            source TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_tokens_block ON tokens(block_id);
    ",
    )?;

    migrate(
        conn,
        5,
        "create_entities",
        "
        CREATE TABLE IF NOT EXISTS entities (
            entity_id INTEGER PRIMARY KEY,
            block_id INTEGER NOT NULL,
            text TEXT NOT NULL,
            category TEXT NOT NULL,
            confidence REAL NOT NULL,
            source TEXT NOT NULL,
            keep BOOLEAN NOT NULL DEFAULT TRUE,
            filter TEXT,
            filter_reason TEXT,
            span_start INTEGER NOT NULL,
            span_end INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_entities_block ON entities(block_id);
        CREATE INDEX IF NOT EXISTS idx_entities_category ON entities(category);
    ",
    )?;

    migrate(
        conn,
        6,
        "create_noun_signals",
        "
        CREATE TABLE IF NOT EXISTS noun_signals (
            signal_id INTEGER PRIMARY KEY,
            block_id INTEGER NOT NULL,
            text TEXT NOT NULL,
            pos TEXT NOT NULL,
            syntactic_role TEXT,
            score REAL NOT NULL,
            span_start INTEGER NOT NULL,
            span_end INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_noun_signals_block ON noun_signals(block_id);
    ",
    )?;

    migrate(
        conn,
        7,
        "create_analysis_results",
        "
        CREATE TABLE IF NOT EXISTS analysis_results (
            result_id INTEGER PRIMARY KEY,
            project_id UUID NOT NULL,
            dimension TEXT NOT NULL,
            score REAL,
            data JSON NOT NULL,
            computed_at TIMESTAMP NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_analysis_project ON analysis_results(project_id);
        CREATE INDEX IF NOT EXISTS idx_analysis_dimension ON analysis_results(dimension);
    ",
    )?;

    migrate(
        conn,
        8,
        "create_concerns",
        "
        CREATE TABLE IF NOT EXISTS concerns (
            concern_id INTEGER PRIMARY KEY,
            result_id INTEGER NOT NULL,
            severity TEXT NOT NULL,
            title TEXT NOT NULL,
            description TEXT NOT NULL,
            block_id TEXT,
            section_path TEXT,
            span_start INTEGER,
            span_end INTEGER,
            suggestion TEXT
        );
    ",
    )?;

    Ok(())
}

fn migrate(conn: &Connection, version: i64, name: &str, sql: &str) -> Result<()> {
    let applied: Option<i64> = conn
        .query_row(
            "SELECT version FROM _migrations WHERE version = ?",
            [version],
            |row| row.get(0),
        )
        .ok();

    if applied == Some(version) {
        return Ok(());
    }

    conn.execute_batch(sql)
        .map_err(|e| crate::StudioError::Database(format!("Migration {version} ({name}): {e}")))?;

    conn.execute(
        "INSERT INTO _migrations (version, name) VALUES (?, ?)",
        duckdb::params![version, name],
    )
    .map_err(|e| crate::StudioError::Database(e.to_string()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migrations_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        run_migrations(&conn).unwrap();
    }

    #[test]
    fn test_migrations_create_tables() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM information_schema.tables WHERE table_name IN 
                 ('projects', 'documents', 'semantic_blocks', 'tokens', 'entities', 
                  'noun_signals', 'analysis_results', 'concerns')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 8);
    }
}
