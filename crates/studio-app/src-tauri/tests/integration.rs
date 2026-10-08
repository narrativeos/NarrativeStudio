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
use studio_analysis::{
    document_hash, run_assessment, run_t0_analysis, run_t1_analysis, Assessment, T0Stats, T1Stats,
};
use studio_core::concern::Severity;
use studio_core::project::Project;
use studio_import::parse_semantic_result;
use studio_storage::{
    create, invalidate_project, list, list_concerns, load_cached, load_document, run_migrations,
    save_concerns, save_document, save_result,
};

/// Path to the semantic_result.json fixture used by these tests.
///
/// Defaults to the committed repo fixture (`tests/fixtures/`), so the suite runs
/// on any machine and on CI. `NARRATIVE_TEST_FILE` overrides it — point it at a
/// full TraceView `semantic_result.json` to smoke-test against real data.
fn test_file_path() -> String {
    std::env::var("NARRATIVE_TEST_FILE").unwrap_or_else(|_| {
        format!(
            "{}/tests/fixtures/traceview_semantic_sample.json",
            env!("CARGO_MANIFEST_DIR")
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
    println!(
        "✓ Created project: {} ({})",
        project.name, project.project_id
    );

    // 4. Save the document
    save_document(&conn, project.project_id, &doc, None).expect("Failed to save document");
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
    assert!(!t0.top_words.is_empty(), "Top words should not be empty");
    println!(
        "✓ T0: {} words, {} blocks, top word: {}",
        t0.word_count,
        t0.block_count,
        t0.top_words.first().map(|(w, _)| w.as_str()).unwrap_or("?")
    );

    // 8. Run T1 analysis
    let t1 = run_t1_analysis(&doc).expect("T1 analysis failed");
    assert_eq!(
        t1.section_count as usize,
        t1.sections.len(),
        "Section count should match sections list length"
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
    assert!(!first.content.is_empty(), "First block should have content");

    // Check tokens exist
    let total_tokens: usize = doc.blocks.iter().map(|b| b.tokens.len()).sum();
    assert!(total_tokens > 0, "Should have tokens");

    // The committed fixture is a 12-block window carrying real entity coverage
    // (PERSON + LOCATION), so the entity/structural paths of the analysis layer
    // are exercised, not just word counting. Guard against fixture rot.
    assert!(
        doc.blocks.len() >= 10,
        "Fixture should carry at least 10 blocks, got {}",
        doc.blocks.len()
    );
    let total_entities: usize = doc.blocks.iter().map(|b| b.entities.len()).sum();
    assert!(
        total_entities > 50,
        "Fixture should carry rich entity coverage, got {total_entities}"
    );
    let has_person = doc
        .blocks
        .iter()
        .flat_map(|b| b.entities.iter())
        .any(|e| e.category == studio_core::entity::EntityCategory::Person);
    assert!(has_person, "Fixture should include PERSON entities");

    println!(
        "✓ Structure: {} blocks, {} total tokens, {} entities",
        doc.blocks.len(),
        total_tokens,
        total_entities
    );
}

/// The analysis cache as the app actually uses it: hash the document loaded from
/// the database, store the computed dimensions under that hash, and read them
/// back on the next open instead of recomputing.
#[test]
fn test_analysis_cache_roundtrip() {
    let file_path = test_file_path();
    let content = std::fs::read_to_string(&file_path).expect("Failed to read test file");
    let doc = parse_semantic_result(&content).expect("Failed to parse semantic result");

    let conn = Connection::open_in_memory().expect("Failed to open in-memory DuckDB");
    run_migrations(&conn).expect("Failed to run migrations");
    let project = create(&conn, &Project::new("Cache Project")).expect("create project");
    save_document(&conn, project.project_id, &doc, None).expect("save document");

    // The cache key is computed from the *loaded* document, so what matters is
    // that loading the same rows twice yields the same fingerprint. If this were
    // unstable the cache would miss on every launch.
    let loaded = load_document(&conn, doc.doc_id).expect("load document");
    let hash = document_hash(&loaded).expect("hash document");
    let again = load_document(&conn, doc.doc_id).expect("load document again");
    assert_eq!(hash, document_hash(&again).unwrap(), "hash must be stable");
    assert_eq!(hash.len(), 64, "hash must be lowercase hex SHA-256");
    println!("✓ Input hash stable across loads: {}…", &hash[..12]);

    // First run: nothing stored yet, so every dimension misses.
    assert!(
        load_cached::<T0Stats>(&conn, project.project_id, "t0", &hash)
            .unwrap()
            .is_none()
    );

    // Compute, then persist exactly the way the command layer does.
    let t0 = run_t0_analysis(&loaded).expect("T0 failed");
    let t1 = run_t1_analysis(&loaded).expect("T1 failed");
    let assessment = run_assessment(&t0, &t1);
    save_result(
        &conn,
        project.project_id,
        "t0",
        "t0",
        &hash,
        None,
        &serde_json::to_string(&t0).unwrap(),
    )
    .expect("save t0");
    save_result(
        &conn,
        project.project_id,
        "t1",
        "t1",
        &hash,
        None,
        &serde_json::to_string(&t1).unwrap(),
    )
    .expect("save t1");
    let assessment_id = save_result(
        &conn,
        project.project_id,
        "assessment",
        "t1",
        &hash,
        Some(assessment.overall as f32),
        &serde_json::to_string(&assessment).unwrap(),
    )
    .expect("save assessment");
    let concerns: Vec<studio_core::concern::Concern> = assessment
        .recommendations
        .iter()
        .map(|r| studio_core::concern::Concern {
            severity: match r.severity.as_str() {
                "high" => Severity::High,
                "low" => Severity::Low,
                _ => Severity::Medium,
            },
            title: r.title.clone(),
            description: r.detail.clone(),
            location: studio_core::concern::ConcernLocation {
                block_id: None,
                section_path: None,
                span_start: None,
                span_end: None,
            },
            suggestion: None,
        })
        .collect();
    save_concerns(&conn, assessment_id, &concerns).expect("save concerns");

    // Second open: every dimension comes back identical to what was computed.
    let cached_t0 = load_cached::<T0Stats>(&conn, project.project_id, "t0", &hash)
        .unwrap()
        .expect("t0 cache hit");
    assert_eq!(
        serde_json::to_string(&cached_t0).unwrap(),
        serde_json::to_string(&t0).unwrap(),
        "cached T0 must round-trip exactly"
    );
    let cached_t1 = load_cached::<T1Stats>(&conn, project.project_id, "t1", &hash)
        .unwrap()
        .expect("t1 cache hit");
    assert_eq!(cached_t1.section_count, t1.section_count);
    let cached_assessment =
        load_cached::<Assessment>(&conn, project.project_id, "assessment", &hash)
            .unwrap()
            .expect("assessment cache hit");
    assert_eq!(cached_assessment.overall, assessment.overall);
    println!(
        "✓ Cache hit for t0/t1/assessment ({} recommendations)",
        assessment.recommendations.len()
    );

    // The recommendations are queryable without re-running the analysis.
    let stored = list_concerns(&conn, project.project_id).expect("list concerns");
    assert_eq!(stored.len(), assessment.recommendations.len());
    println!("✓ {} concerns persisted", stored.len());

    // A different input hash (i.e. edited text) must not read these rows.
    assert!(
        load_cached::<T0Stats>(&conn, project.project_id, "t0", "stale-hash")
            .unwrap()
            .is_none()
    );

    // "重新分析" clears everything for the project.
    assert_eq!(invalidate_project(&conn, project.project_id).unwrap(), 3);
    assert!(
        load_cached::<T0Stats>(&conn, project.project_id, "t0", &hash)
            .unwrap()
            .is_none()
    );
    assert!(list_concerns(&conn, project.project_id).unwrap().is_empty());
    println!("✓ invalidate_project clears results and concerns");
}
