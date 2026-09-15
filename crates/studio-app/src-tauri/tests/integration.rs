//! Integration test: full import + analysis flow
//!
//! Tests the complete pipeline:
//! 1. Parse a semantic_result.json file
//! 2. Create a project in DuckDB
//! 3. Save the document
//! 4. List projects
//! 5. Run T0 analysis
//! 6. Run T1 analysis

use duckdb::Connection;
use studio_analysis::{run_t0_analysis, run_t1_analysis};
use studio_core::project::Project;
use studio_import::parse_semantic_result;
use studio_storage::{create, list, load_document, run_migrations, save_document};

/// Path to a real semantic_result.json for testing.
/// Override with NARRATIVE_TEST_FILE env var if needed.
fn test_file_path() -> String {
    std::env::var("NARRATIVE_TEST_FILE").unwrap_or_else(|_| {
        format!(
            "{}/.TraceView/ovgj/semantic/semantic_result.json",
            std::env::var("HOME").unwrap()
        )
    })
}

#[test]
fn test_full_import_and_analysis_flow() {
    let file_path = test_file_path();
    println!("Using test file: {file_path}");

    // 1. Read and parse the semantic_result.json
    let content = std::fs::read_to_string(&file_path)
        .unwrap_or_else(|e| panic!("Failed to read test file: {e}"));
    let doc = parse_semantic_result(&content)
        .unwrap_or_else(|e| panic!("Failed to parse semantic result: {e}"));
    println!("✓ Parsed document: {} blocks", doc.blocks.len());

    // 2. Set up in-memory DuckDB
    let conn = Connection::open_in_memory().expect("Failed to open in-memory DuckDB");
    run_migrations(&conn).expect("Failed to run migrations");
    println!("✓ Migrations applied");

    // 3. Create a project
    let project = Project::new("Test Import Project");
    let project = create(&conn, &project).expect("Failed to create project");
    println!("✓ Created project: {} ({})", project.name, project.project_id);

    // 4. Save the document
    save_document(&conn, project.project_id, &doc)
        .expect("Failed to save document");
    println!("✓ Document saved");

    // 5. List projects
    let projects = list(&conn).expect("Failed to list projects");
    assert!(!projects.is_empty(), "Project list should not be empty");
    assert_eq!(projects[0].name, "Test Import Project");
    println!("✓ Listed {} project(s)", projects.len());

    // 6. Load the document back (use doc.doc_id, not project_id)
    let loaded = load_document(&conn, doc.doc_id).expect("Failed to load document");
    assert_eq!(
        loaded.blocks.len(),
        doc.blocks.len(),
        "Block count mismatch after load"
    );
    println!("✓ Document loaded back: {} blocks", loaded.blocks.len());

    // 7. Run T0 analysis
    let t0 = run_t0_analysis(&doc).expect("T0 analysis failed");
    assert!(t0.word_count > 0, "Word count should be > 0");
    assert!(t0.block_count > 0, "Block count should be > 0");
    assert!(
        !t0.top_words.is_empty(),
        "Top words should not be empty"
    );
    println!(
        "✓ T0: {} words, {} blocks, top word: {}",
        t0.word_count,
        t0.block_count,
        t0.top_words.first().map(|(w, _)| w.as_str()).unwrap_or("?")
    );

    // 8. Run T1 analysis
    let t1 = run_t1_analysis(&doc).expect("T1 analysis failed");
    assert!(
        t1.section_count >= 0,
        "Section count should be >= 0"
    );
    assert!(
        !t1.block_type_distribution.is_empty(),
        "Block type distribution should not be empty"
    );
    println!(
        "✓ T1: {} sections, {} block types",
        t1.section_count,
        t1.block_type_distribution.len()
    );

    println!("\n=== ALL TESTS PASSED ===");
}

#[test]
fn test_parse_semantic_result_structure() {
    let file_path = test_file_path();
    let content = std::fs::read_to_string(&file_path)
        .unwrap_or_else(|e| panic!("Failed to read test file: {e}"));
    let doc = parse_semantic_result(&content).expect("Parse failed");

    // Verify document structure
    assert!(!doc.blocks.is_empty(), "Should have blocks");

    // Check first block has content
    let first = &doc.blocks[0];
    assert!(
        !first.content.is_empty(),
        "First block should have content"
    );

    // Check tokens exist
    let total_tokens: usize = doc.blocks.iter().map(|b| b.tokens.len()).sum();
    assert!(total_tokens > 0, "Should have tokens");
    println!(
        "✓ Structure: {} blocks, {} total tokens",
        doc.blocks.len(),
        total_tokens
    );
}
