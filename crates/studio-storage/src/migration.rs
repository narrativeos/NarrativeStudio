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
            source_block_id VARCHAR PRIMARY KEY,
            doc_id UUID NOT NULL,
            seq INTEGER NOT NULL,
            content TEXT NOT NULL,
            section_path TEXT NOT NULL,
            block_type TEXT NOT NULL,
            title TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_semantic_blocks_doc ON semantic_blocks(doc_id);
    ",
    )?;

    migrate(
        conn,
        4,
        "create_tokens",
        "
        CREATE TABLE IF NOT EXISTS tokens (
            token_id UUID PRIMARY KEY,
            source_block_id VARCHAR NOT NULL,
            seq INTEGER NOT NULL,
            text TEXT NOT NULL,
            pos TEXT NOT NULL,
            confidence REAL NOT NULL,
            span_start INTEGER NOT NULL,
            span_end INTEGER NOT NULL,
            source TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_tokens_block ON tokens(source_block_id);
    ",
    )?;

    migrate(
        conn,
        5,
        "create_entities",
        "
        CREATE TABLE IF NOT EXISTS entities (
            entity_id UUID PRIMARY KEY,
            source_block_id VARCHAR NOT NULL,
            seq INTEGER NOT NULL,
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
        CREATE INDEX IF NOT EXISTS idx_entities_block ON entities(source_block_id);
        CREATE INDEX IF NOT EXISTS idx_entities_category ON entities(category);
    ",
    )?;

    migrate(
        conn,
        6,
        "create_noun_signals",
        "
        CREATE TABLE IF NOT EXISTS noun_signals (
            signal_id UUID PRIMARY KEY,
            source_block_id VARCHAR NOT NULL,
            seq INTEGER NOT NULL,
            text TEXT NOT NULL,
            pos TEXT NOT NULL,
            syntactic_role TEXT,
            score REAL NOT NULL,
            span_start INTEGER NOT NULL,
            span_end INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_noun_signals_block ON noun_signals(source_block_id);
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

    migrate(
        conn,
        9,
        "add_source_path_to_projects",
        "
        ALTER TABLE projects ADD COLUMN IF NOT EXISTS source_path TEXT;
    ",
    )?;

    // v10: re-key the block tables by the TraceView source UUID.
    //
    // The original schema used a per-document integer `block_id` (and per-document
    // integer counters for token/entity/signal ids), which collides across
    // documents and breaks multi-project analysis. We now key blocks by their
    // globally-unique `source_block_id` (the UUID from semantic_result.json) and
    // use UUID primary keys + a `seq` column for the child tables. Existing rows
    // cannot be backfilled (the source UUID was never stored), so the block
    // tables are dropped and recreated; the user re-imports their projects.
    migrate(
        conn,
        10,
        "rekey_blocks_by_source_block_id",
        "
        DROP TABLE IF EXISTS noun_signals;
        DROP TABLE IF EXISTS entities;
        DROP TABLE IF EXISTS tokens;
        DROP TABLE IF EXISTS semantic_blocks;

        CREATE TABLE IF NOT EXISTS semantic_blocks (
            source_block_id VARCHAR PRIMARY KEY,
            doc_id UUID NOT NULL,
            seq INTEGER NOT NULL,
            content TEXT NOT NULL,
            section_path TEXT NOT NULL,
            block_type TEXT NOT NULL,
            title TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_semantic_blocks_doc ON semantic_blocks(doc_id);

        CREATE TABLE IF NOT EXISTS tokens (
            token_id UUID PRIMARY KEY,
            source_block_id VARCHAR NOT NULL,
            seq INTEGER NOT NULL,
            text TEXT NOT NULL,
            pos TEXT NOT NULL,
            confidence REAL NOT NULL,
            span_start INTEGER NOT NULL,
            span_end INTEGER NOT NULL,
            source TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_tokens_block ON tokens(source_block_id);

        CREATE TABLE IF NOT EXISTS entities (
            entity_id UUID PRIMARY KEY,
            source_block_id VARCHAR NOT NULL,
            seq INTEGER NOT NULL,
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
        CREATE INDEX IF NOT EXISTS idx_entities_block ON entities(source_block_id);
        CREATE INDEX IF NOT EXISTS idx_entities_category ON entities(category);

        CREATE TABLE IF NOT EXISTS noun_signals (
            signal_id UUID PRIMARY KEY,
            source_block_id VARCHAR NOT NULL,
            seq INTEGER NOT NULL,
            text TEXT NOT NULL,
            pos TEXT NOT NULL,
            syntactic_role TEXT,
            score REAL NOT NULL,
            span_start INTEGER NOT NULL,
            span_end INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_noun_signals_block ON noun_signals(source_block_id);
    ",
    )?;

    // v11: make stored analysis results reusable.
    //
    // `analysis_results` has existed since v7 but nothing ever wrote to it, so
    // every page load recomputed T0/T1 from the imported blocks. Two columns
    // turn it into a cache that can be invalidated safely:
    //   tier        — which pipeline stage produced the row ('t0' | 't1' | 't2');
    //   input_hash  — SHA-256 of the analysed content (studio_analysis::hash),
    //                 so a row is only ever reused for the exact input it came
    //                 from, and a changed document simply misses.
    // The unique index makes re-analysis idempotent instead of accumulating one
    // row per run per dimension. The sequences supply the integer primary keys
    // that the v7/v8 tables declare but never generated.
    //
    // Both columns are nullable: DuckDB rejects `ADD COLUMN` with a constraint
    // ("Adding columns with constraints not yet supported"), so `NOT NULL
    // DEFAULT 't0'` is not an option. Every row written through `analysis_repo`
    // fills both, and readers fall back to a default for pre-v11 rows.
    migrate(
        conn,
        11,
        "analysis_result_cache",
        "
        ALTER TABLE analysis_results ADD COLUMN IF NOT EXISTS tier TEXT;
        ALTER TABLE analysis_results ADD COLUMN IF NOT EXISTS input_hash TEXT;

        CREATE SEQUENCE IF NOT EXISTS seq_analysis_results START 1;
        CREATE SEQUENCE IF NOT EXISTS seq_concerns START 1;

        CREATE UNIQUE INDEX IF NOT EXISTS idx_analysis_result_unique
            ON analysis_results(project_id, dimension, input_hash);
    ",
    )?;

    // v12: generated reports.
    //
    // A report is stored as the rendered Markdown rather than re-derived on
    // demand: an export has to stay reproducible after the analysis behind it was
    // re-run or invalidated, and `exported_path` records the file the user
    // actually saved. `report_id` is a UUID (as in `studio_core::Report`) because
    // reports are only ever referenced directly, never renumbered.
    migrate(
        conn,
        12,
        "create_reports",
        "
        CREATE TABLE IF NOT EXISTS reports (
            report_id UUID PRIMARY KEY,
            project_id UUID NOT NULL,
            title TEXT,
            markdown TEXT NOT NULL,
            exported_path TEXT,
            created_at TIMESTAMP NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_reports_project ON reports(project_id);
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
                  'noun_signals', 'analysis_results', 'concerns', 'reports')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 9);
    }

    #[test]
    fn test_v11_analysis_cache_schema() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();

        // The two columns the result cache is keyed on must exist.
        let cols: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM information_schema.columns \
                 WHERE table_name = 'analysis_results' \
                   AND column_name IN ('tier', 'input_hash')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(cols, 2);

        // The sequences must hand out distinct ids (the v7/v8 tables have no
        // auto-increment of their own).
        let a: i64 = conn
            .query_row("SELECT nextval('seq_analysis_results')", [], |r| r.get(0))
            .unwrap();
        let b: i64 = conn
            .query_row("SELECT nextval('seq_analysis_results')", [], |r| r.get(0))
            .unwrap();
        assert_ne!(a, b);

        // The unique index makes a second insert for the same
        // (project, dimension, input_hash) fail rather than duplicate a row.
        let project = "11111111-1111-1111-1111-111111111111";
        let insert = "INSERT INTO analysis_results \
                      (result_id, project_id, dimension, tier, input_hash, data, computed_at) \
                      VALUES (?, ?, ?, ?, ?, ?, ?)";
        conn.execute(
            insert,
            duckdb::params![a, project, "t0", "t0", "h1", "{}", "2026-01-01 00:00:00"],
        )
        .unwrap();
        let duplicate = conn.execute(
            insert,
            duckdb::params![b, project, "t0", "t0", "h1", "{}", "2026-01-01 00:00:00"],
        );
        assert!(duplicate.is_err(), "unique index must reject duplicates");

        // A different input_hash is a different row, not a conflict.
        conn.execute(
            insert,
            duckdb::params![b, project, "t0", "t0", "h2", "{}", "2026-01-01 00:00:00"],
        )
        .unwrap();
    }
}
