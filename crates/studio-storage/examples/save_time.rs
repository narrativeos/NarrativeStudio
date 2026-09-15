//! Measure save_document write throughput (in-memory DuckDB, no file lock).
use std::time::Instant;

use duckdb::Connection;
use studio_core::document::{DocumentData, SemanticBlock, Token};
use studio_storage::{run_migrations, save_document};

fn main() {
    let conn = Connection::open_in_memory().unwrap();
    run_migrations(&conn).unwrap();
    let project_id = uuid::Uuid::new_v4();

    let num_blocks = 2000;
    let tokens_per_block = 68; // ~136k tokens, matches real import ratio
    let mut blocks = Vec::with_capacity(num_blocks);
    for b in 0..num_blocks {
        let tokens: Vec<Token> = (0..tokens_per_block)
            .map(|t| Token {
                text: format!("token_{}_{}", b, t),
                pos: "NN".to_string(),
                confidence: 0.9,
                span: (t * 2, t * 2 + 2),
                source: "test".to_string(),
            })
            .collect();
        blocks.push(SemanticBlock {
            source_block_id: uuid::Uuid::new_v4().to_string(),
            content: format!("block content {}", b),
            section_path: "test".to_string(),
            block_type: "paragraph".to_string(),
            title: None,
            tokens,
            entities: vec![],
            noun_signals: vec![],
        });
    }
    let doc = DocumentData {
        doc_id: uuid::Uuid::new_v4(),
        title: "Test".to_string(),
        blocks,
        total_word_count: 100000,
        total_char_count: 500000,
    };

    let total_tokens: usize = doc.blocks.iter().map(|b| b.tokens.len()).sum();
    println!("blocks: {}, tokens: {}", doc.blocks.len(), total_tokens);

    let t0 = Instant::now();
    save_document(&conn, project_id, &doc, None).unwrap();
    let elapsed = t0.elapsed().as_secs_f64();
    println!("save_document: {:.2}s", elapsed);
    println!(
        "throughput: {:.0} tokens/sec",
        total_tokens as f64 / elapsed
    );
}
