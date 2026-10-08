//! Integration test: full import + analysis flow
//!
//! Tests the complete pipeline:
//! 1. Parse a semantic_result.json file
//! 2. Create a project in DuckDB
//! 3. Save the document
//! 4. List projects
//! 5. Run T0 analysis
//! 6. Run T1 analysis
//! 7. Run plot analysis
//! 8. Run pacing analysis
//! 9. Run conflict analysis
//! 10. Run theme analysis
//! 11. Run setting analysis
//! 12. Run cliché analysis
//! 13. Run dialogue analysis
//! 14. Run explicit language analysis
//! 15. Run story structure analysis

use chrono::{TimeZone, Utc};
use duckdb::Connection;
use studio_analysis::{
    document_hash, run_assessment, run_cliche_analysis, run_conflict_analysis,
    run_dialogue_analysis, run_explicit_analysis, run_pacing_analysis, run_plot_analysis,
    run_setting_analysis, run_structure_analysis, run_t0_analysis, run_t1_analysis,
    run_theme_analysis, run_timeline_analysis, Assessment, ClicheStats, ConflictStats,
    DialogueStats, ExplicitStats, PacingStats, PlotStats, SettingStats, StructureStats, T0Stats,
    T1Stats, ThemeStats, TimelineStats,
};
use studio_core::concern::Severity;
use studio_core::project::Project;
use studio_import::parse_semantic_result;
use studio_report::{render_markdown, ReportInput};
use studio_storage::{
    create, get_report, invalidate_project, list, list_concerns, load_cached, load_document,
    mark_exported, run_migrations, save_concerns, save_document, save_report, save_result,
    StoredReport,
};
use uuid::Uuid;

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
    let plot = run_plot_analysis(&loaded).expect("plot analysis failed");
    let pacing = run_pacing_analysis(&loaded).expect("pacing analysis failed");
    let conflict = run_conflict_analysis(&loaded).expect("conflict analysis failed");
    let theme = run_theme_analysis(&loaded).expect("theme analysis failed");
    let setting = run_setting_analysis(&loaded).expect("setting analysis failed");
    let cliche = run_cliche_analysis(&loaded).expect("cliché analysis failed");
    let dialogue = run_dialogue_analysis(&loaded).expect("dialogue analysis failed");
    let explicit = run_explicit_analysis(&loaded).expect("explicit language analysis failed");
    let structure =
        run_structure_analysis(&loaded, &pacing, &plot).expect("story structure analysis failed");
    let timeline = run_timeline_analysis(&loaded).expect("time line analysis failed");
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
    save_result(
        &conn,
        project.project_id,
        "plot",
        "t1",
        &hash,
        Some(plot.coherence_score as f32),
        &serde_json::to_string(&plot).unwrap(),
    )
    .expect("save plot");
    save_result(
        &conn,
        project.project_id,
        "pacing",
        "t1",
        &hash,
        Some(pacing.pace_score as f32),
        &serde_json::to_string(&pacing).unwrap(),
    )
    .expect("save pacing");
    save_result(
        &conn,
        project.project_id,
        "conflict",
        "t1",
        &hash,
        conflict.conflict_score.map(|s| s as f32),
        &serde_json::to_string(&conflict).unwrap(),
    )
    .expect("save conflict");
    save_result(
        &conn,
        project.project_id,
        "theme",
        "t1",
        &hash,
        theme.theme_score.map(|s| s as f32),
        &serde_json::to_string(&theme).unwrap(),
    )
    .expect("save theme");
    save_result(
        &conn,
        project.project_id,
        "setting",
        "t1",
        &hash,
        setting.setting_score.map(|s| s as f32),
        &serde_json::to_string(&setting).unwrap(),
    )
    .expect("save setting");
    save_result(
        &conn,
        project.project_id,
        "cliche",
        "t1",
        &hash,
        cliche.cliche_score.map(|s| s as f32),
        &serde_json::to_string(&cliche).unwrap(),
    )
    .expect("save cliche");
    // Dialogue carries no score: the row records the absence, not a 0.
    save_result(
        &conn,
        project.project_id,
        "dialogue",
        "t1",
        &hash,
        None,
        &serde_json::to_string(&dialogue).unwrap(),
    )
    .expect("save dialogue");
    // Explicit language carries no score either: what a text contains is not a mark
    // against it, so the row records the absence rather than a 0.
    save_result(
        &conn,
        project.project_id,
        "explicit",
        "t1",
        &hash,
        None,
        &serde_json::to_string(&explicit).unwrap(),
    )
    .expect("save explicit");
    // Story structure carries no score either: fitting three-act shape is not a virtue,
    // so the row keeps the description without inventing a mark for it.
    save_result(
        &conn,
        project.project_id,
        "structure",
        "t1",
        &hash,
        None,
        &serde_json::to_string(&structure).unwrap(),
    )
    .expect("save structure");
    // The time line carries no score either: a story that never writes a date is not
    // keeping time badly, so the row keeps the description without a mark.
    save_result(
        &conn,
        project.project_id,
        "timeline",
        "t1",
        &hash,
        None,
        &serde_json::to_string(&timeline).unwrap(),
    )
    .expect("save timeline");
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
    let cached_plot = load_cached::<PlotStats>(&conn, project.project_id, "plot", &hash)
        .unwrap()
        .expect("plot cache hit");
    assert_eq!(
        serde_json::to_string(&cached_plot).unwrap(),
        serde_json::to_string(&plot).unwrap(),
        "cached plot analysis must round-trip exactly"
    );
    let cached_pacing = load_cached::<PacingStats>(&conn, project.project_id, "pacing", &hash)
        .unwrap()
        .expect("pacing cache hit");
    // Not a string comparison: the payload lives in a JSON column, and DuckDB
    // re-formats doubles on the way out, so a standard deviation can come back one
    // unit in the last place away from what was written. The numbers are what have
    // to survive the round trip.
    assert_eq!(cached_pacing.segment_count, pacing.segment_count);
    assert_eq!(cached_pacing.verdict, pacing.verdict);
    assert_eq!(cached_pacing.points.len(), pacing.points.len());
    for (cached, fresh) in cached_pacing.points.iter().zip(pacing.points.iter()) {
        assert_eq!(cached.section, fresh.section);
        assert!((cached.tension - fresh.tension).abs() < 1e-9);
    }
    assert!(
        (cached_pacing.pace_score - pacing.pace_score).abs() < 1e-6,
        "pace score must survive the cache"
    );
    let cached_conflict =
        load_cached::<ConflictStats>(&conn, project.project_id, "conflict", &hash)
            .unwrap()
            .expect("conflict cache hit");
    assert_eq!(cached_conflict.segment_count, conflict.segment_count);
    assert_eq!(cached_conflict.verdict, conflict.verdict);
    assert_eq!(cached_conflict.total_hits, conflict.total_hits);
    assert_eq!(cached_conflict.types.len(), conflict.types.len());
    assert_eq!(
        cached_conflict.conflict_score.is_some(),
        conflict.conflict_score.is_some(),
        "an unscored conflict must stay unscored in the cache"
    );
    if let (Some(cached), Some(fresh)) = (cached_conflict.conflict_score, conflict.conflict_score) {
        assert!(
            (cached - fresh).abs() < 1e-6,
            "conflict score must survive the cache"
        );
    }
    let cached_theme = load_cached::<ThemeStats>(&conn, project.project_id, "theme", &hash)
        .unwrap()
        .expect("theme cache hit");
    assert_eq!(cached_theme.source, theme.source);
    assert_eq!(cached_theme.candidate_count, theme.candidate_count);
    assert_eq!(cached_theme.total_mentions, theme.total_mentions);
    assert_eq!(cached_theme.themes.len(), theme.themes.len());
    for (cached, fresh) in cached_theme.themes.iter().zip(theme.themes.iter()) {
        assert_eq!(cached.term, fresh.term);
        assert_eq!(cached.mentions, fresh.mentions);
        assert_eq!(cached.curve, fresh.curve);
    }
    assert_eq!(
        cached_theme.theme_score.is_some(),
        theme.theme_score.is_some(),
        "an unscored theme list must stay unscored in the cache"
    );
    if let (Some(cached), Some(fresh)) = (cached_theme.theme_score, theme.theme_score) {
        assert!(
            (cached - fresh).abs() < 1e-6,
            "theme score must survive the cache"
        );
    }
    let cached_setting = load_cached::<SettingStats>(&conn, project.project_id, "setting", &hash)
        .unwrap()
        .expect("setting cache hit");
    assert_eq!(cached_setting.scene_count, setting.scene_count);
    assert_eq!(cached_setting.place_count, setting.place_count);
    assert_eq!(cached_setting.total_env_hits, setting.total_env_hits);
    assert_eq!(cached_setting.places.len(), setting.places.len());
    for (cached, fresh) in cached_setting.places.iter().zip(setting.places.iter()) {
        assert_eq!(cached.name, fresh.name);
        assert_eq!(cached.mentions, fresh.mentions);
        assert_eq!(cached.curve, fresh.curve);
    }
    assert_eq!(
        cached_setting.setting_score.is_some(),
        setting.setting_score.is_some(),
        "a document with nothing to grade must stay unscored in the cache"
    );
    if let (Some(cached), Some(fresh)) = (cached_setting.setting_score, setting.setting_score) {
        assert!(
            (cached - fresh).abs() < 1e-6,
            "setting score must survive the cache"
        );
    }
    let cached_cliche = load_cached::<ClicheStats>(&conn, project.project_id, "cliche", &hash)
        .unwrap()
        .expect("cliche cache hit");
    assert_eq!(cached_cliche.total_hits, cliche.total_hits);
    assert_eq!(cached_cliche.distinct_phrases, cliche.distinct_phrases);
    assert_eq!(cached_cliche.phrases.len(), cliche.phrases.len());
    assert_eq!(cached_cliche.hits.len(), cliche.hits.len());
    for (cached, fresh) in cached_cliche.phrases.iter().zip(cliche.phrases.iter()) {
        assert_eq!(cached.phrase, fresh.phrase);
        assert_eq!(cached.count, fresh.count);
        assert_eq!(cached.variants, fresh.variants);
    }
    assert_eq!(
        cached_cliche.cliche_score.is_some(),
        cliche.cliche_score.is_some(),
        "a document too short to rate must stay unscored in the cache"
    );
    if let (Some(cached), Some(fresh)) = (cached_cliche.cliche_score, cliche.cliche_score) {
        assert!(
            (cached - fresh).abs() < 1e-6,
            "cliche score must survive the cache"
        );
    }
    let cached_dialogue =
        load_cached::<DialogueStats>(&conn, project.project_id, "dialogue", &hash)
            .unwrap()
            .expect("dialogue cache hit");
    assert_eq!(cached_dialogue.char_count, dialogue.char_count);
    assert_eq!(cached_dialogue.dialogue_chars, dialogue.dialogue_chars);
    assert_eq!(cached_dialogue.unit_count, dialogue.unit_count);
    assert_eq!(cached_dialogue.points.len(), dialogue.points.len());
    assert_eq!(cached_dialogue.samples.len(), dialogue.samples.len());
    assert!(
        (cached_dialogue.dialogue_share - dialogue.dialogue_share).abs() < 1e-9,
        "dialogue share must survive the cache"
    );
    let cached_explicit =
        load_cached::<ExplicitStats>(&conn, project.project_id, "explicit", &hash)
            .unwrap()
            .expect("explicit cache hit");
    assert_eq!(cached_explicit.total_hits, explicit.total_hits);
    assert_eq!(cached_explicit.distinct_terms, explicit.distinct_terms);
    assert_eq!(cached_explicit.hits.len(), explicit.hits.len());
    assert_eq!(cached_explicit.terms.len(), explicit.terms.len());
    assert_eq!(cached_explicit.advisory, explicit.advisory);
    assert_eq!(cached_explicit.tiers, explicit.tiers);
    assert!(
        (cached_explicit.per_10k - explicit.per_10k).abs() < 1e-9,
        "explicit rate must survive the cache"
    );
    let cached_assessment =
        load_cached::<Assessment>(&conn, project.project_id, "assessment", &hash)
            .unwrap()
            .expect("assessment cache hit");
    assert_eq!(cached_assessment.overall, assessment.overall);
    let cached_structure =
        load_cached::<StructureStats>(&conn, project.project_id, "structure", &hash)
            .unwrap()
            .expect("structure cache hit");
    assert_eq!(cached_structure.landmarks.len(), structure.landmarks.len());
    assert_eq!(cached_structure.acts.len(), structure.acts.len());
    assert_eq!(cached_structure.models.len(), structure.models.len());
    assert_eq!(cached_structure.issues.len(), structure.issues.len());
    assert_eq!(cached_structure.segment_count, structure.segment_count);
    assert_eq!(cached_structure.chunked, structure.chunked);
    assert_eq!(cached_structure.char_count, structure.char_count);
    assert_eq!(cached_structure.turn_count, structure.turn_count);
    assert_eq!(cached_structure.readable, structure.readable);
    assert_eq!(cached_structure.note, structure.note);
    if let (Some(cached), Some(fresh)) = (cached_structure.models.first(), structure.models.first())
    {
        assert_eq!(cached.model, fresh.model);
        assert_eq!(cached.fit, fresh.fit);
        assert_eq!(cached.beats.len(), fresh.beats.len());
        assert_eq!(cached.found, fresh.found);
        assert_eq!(cached.displaced, fresh.displaced);
        assert_eq!(cached.missing, fresh.missing);
    }
    // The stored row keeps the absence of a score: the dimension describes a shape,
    // it does not grade one.
    let stored_score: Option<f32> = conn
        .query_row(
            "SELECT score FROM analysis_results WHERE dimension = 'structure'",
            [],
            |row| row.get(0),
        )
        .expect("structure row");
    assert!(
        stored_score.is_none(),
        "story structure must not be cached with a score"
    );
    let cached_timeline =
        load_cached::<TimelineStats>(&conn, project.project_id, "timeline", &hash)
            .unwrap()
            .expect("timeline cache hit");
    assert_eq!(cached_timeline.markers.len(), timeline.markers.len());
    assert_eq!(cached_timeline.absolute_count, timeline.absolute_count);
    assert_eq!(
        cached_timeline.entity_date_count,
        timeline.entity_date_count
    );
    assert_eq!(cached_timeline.segment_count, timeline.segment_count);
    assert_eq!(cached_timeline.chunked, timeline.chunked);
    assert_eq!(cached_timeline.char_count, timeline.char_count);
    assert_eq!(cached_timeline.readable, timeline.readable);
    assert_eq!(cached_timeline.note, timeline.note);
    // The fixture's CJK is a 1:1 substitution, so every date unit in it was scrambled: the
    // dimension has to report that dates exist without pretending it can order them.
    assert_eq!(cached_timeline.entity_date_count, 19);
    assert!(
        cached_timeline.markers.is_empty(),
        "the scrambled fixture must not yield readable dates"
    );
    let stored_timeline_score: Option<f32> = conn
        .query_row(
            "SELECT score FROM analysis_results WHERE dimension = 'timeline'",
            [],
            |row| row.get(0),
        )
        .expect("timeline row");
    assert!(
        stored_timeline_score.is_none(),
        "the time line must not be cached with a score"
    );
    println!(
        "✓ Cache hit for t0/t1/plot/pacing/conflict/theme/setting/cliche/dialogue/explicit/structure/timeline/assessment ({} threads, {} segments, {} conflict hits, {} themes, {} scenes, {} cliché hits, {} speech passages, {} explicit hits, {} landmarks, {} markers, {} recommendations)",
        plot.thread_count,
        pacing.segment_count,
        conflict.total_hits,
        theme.themes.len(),
        setting.scene_count,
        cliche.total_hits,
        dialogue.unit_count,
        explicit.total_hits,
        structure.landmarks.len(),
        timeline.markers.len(),
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

    // "重新分析" clears everything for the project: twelve dimensions plus the assessment
    // derived from them.
    assert_eq!(invalidate_project(&conn, project.project_id).unwrap(), 13);
    assert!(
        load_cached::<T0Stats>(&conn, project.project_id, "t0", &hash)
            .unwrap()
            .is_none()
    );
    assert!(list_concerns(&conn, project.project_id).unwrap().is_empty());
    println!("✓ invalidate_project clears results and concerns");
}

/// Report export end to end: analyse the fixture, render it to Markdown, persist
/// it, and write it to disk — the same path `report_generate` + `report_export`
/// take, minus the Tauri state wrapper.
#[test]
fn test_report_generation_and_export() {
    let content = std::fs::read_to_string(test_file_path()).expect("Failed to read test file");
    let doc = parse_semantic_result(&content).expect("Failed to parse semantic result");

    let conn = Connection::open_in_memory().expect("Failed to open in-memory DuckDB");
    run_migrations(&conn).expect("Failed to run migrations");
    let project = create(&conn, &Project::new("Report Project")).expect("create project");
    save_document(&conn, project.project_id, &doc, None).expect("save document");

    let loaded = load_document(&conn, doc.doc_id).expect("load document");
    let t0 = run_t0_analysis(&loaded).expect("T0 failed");
    let t1 = run_t1_analysis(&loaded).expect("T1 failed");
    let plot = run_plot_analysis(&loaded).expect("plot analysis failed");
    let pacing = run_pacing_analysis(&loaded).expect("pacing analysis failed");
    let conflict = run_conflict_analysis(&loaded).expect("conflict analysis failed");
    let theme = run_theme_analysis(&loaded).expect("theme analysis failed");
    let setting = run_setting_analysis(&loaded).expect("setting analysis failed");
    let cliche = run_cliche_analysis(&loaded).expect("cliché analysis failed");
    let dialogue = run_dialogue_analysis(&loaded).expect("dialogue analysis failed");
    let explicit = run_explicit_analysis(&loaded).expect("explicit language analysis failed");
    let structure =
        run_structure_analysis(&loaded, &pacing, &plot).expect("story structure analysis failed");
    let timeline = run_timeline_analysis(&loaded).expect("time line analysis failed");
    let assessment = run_assessment(&t0, &t1);

    let generated_at = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
    let markdown = render_markdown(&ReportInput {
        project_name: &project.name,
        generated_at,
        t0: &t0,
        t1: &t1,
        plot: &plot,
        pacing: &pacing,
        conflict: &conflict,
        theme: &theme,
        setting: &setting,
        cliche: &cliche,
        dialogue: &dialogue,
        explicit: &explicit,
        structure: &structure,
        timeline: &timeline,
        assessment: &assessment,
    });

    // A real document has to produce a real report: every section, the title, and
    // the same numbers the analysis page shows.
    for heading in [
        "## 总体评估",
        "## 关键建议",
        "## 文本统计",
        "## 文本诊断",
        "## 对话与叙事",
        "## 章节结构",
        "## 叙事分析",
        "## 情节线",
        "## 节奏",
        "## 冲突",
        "## 主题",
        "## 场景/设定",
        "## 陈词滥调",
        "## 显式语言",
        "## 故事结构",
        "## 时间线",
    ] {
        assert!(markdown.contains(heading), "missing section: {heading}");
    }
    // The fixture's date units were scrambled by desensitization, so the time-line section
    // has to report dates it cannot read rather than invent an order: no ordering, and the
    // entity cross-check named.
    assert!(markdown.contains("本次不做时间排序"));
    assert!(markdown.contains("导入的实体标注里有 19 处日期"));
    assert!(markdown.starts_with(&format!("# {} · 叙事分析报告", project.name)));
    assert!(markdown.contains(&format!("**{:.1} / 100**", assessment.overall)));
    assert!(markdown.contains(&format!("| 字数 | {} |", t0.char_count)));
    assert!(markdown.contains(&format!(
        "情节连贯性：**{:.1} / 100**",
        plot.coherence_score
    )));
    assert!(markdown.contains(&format!("节奏评分：**{:.1} / 100**", pacing.pace_score)));
    assert!(
        !markdown.contains("NaN") && !markdown.contains("Inf"),
        "a non-finite number leaked into the report"
    );
    if let Some(section) = t1.sections.first() {
        assert!(
            markdown.contains(&section.path),
            "first section {} missing",
            section.path
        );
    }
    if let Some(line) = plot.plotlines.first() {
        assert!(
            markdown.contains(&line.subject),
            "top plotline {} missing from the report",
            line.subject
        );
    }
    if let Some(point) = pacing.points.first() {
        assert!(
            markdown.contains(&point.section),
            "first pacing segment {} missing from the report",
            point.section
        );
    }
    // The conflict section always renders its type table, scored or not: an
    // unscored document still has to say what was looked for.
    assert!(markdown.contains("### 冲突类型（"));
    assert!(markdown.contains("冲突用词按词面计数"));
    if !conflict.points.is_empty() {
        assert!(
            markdown.contains("| 章节 | 强度 | 命中 | 千字密度 | 主类型 |"),
            "conflict curve table missing from the report"
        );
    }
    // The theme section names the vocabulary it read, whether or not it scored:
    // a list rebuilt from POS tags is not as trustworthy as one from noun signals.
    assert!(markdown.contains("词汇来源：") || markdown.contains("没有可用于主题分析的词汇"));
    if let Some(top) = theme.themes.first() {
        assert!(
            markdown.contains(&top.term),
            "top theme {} missing from the report",
            top.term
        );
        assert!(markdown.contains("### 主题清单（"));
    }
    // The setting section prints its scene list whenever there are scenes, and the
    // limit of the method either way: a place named outside the entity model shows
    // up only as description.
    if !setting.scenes.is_empty() {
        assert!(markdown.contains("### 场景清单（"));
    }
    assert!(
        markdown.contains("是否真的换了场景需要人来核对") || markdown.contains("未识别到场景"),
        "setting section missing its caveat"
    );
    if let Some(place) = setting.places.first() {
        assert!(
            markdown.contains(&place.name),
            "busiest place {} missing from the report",
            place.name
        );
    }
    // The cliché section always states the limit of the method: a miss means the
    // library found nothing, not that the prose is fresh.
    assert!(markdown.contains("由人来判断"));
    if let Some(phrase) = cliche.phrases.first() {
        assert!(
            markdown.contains("### 高频套话（"),
            "cliche phrase list missing from the report"
        );
        assert!(
            markdown.contains(&phrase.note),
            "replacement note for {} missing",
            phrase.phrase
        );
    } else {
        assert!(
            markdown.contains("没有命中词库里的表达"),
            "an empty cliché pass still has to say so"
        );
    }
    // The dialogue section states the split and the limit of the method: what counts
    // as speech is decided by quotation marks and the dash convention, not by reading.
    assert!(
        markdown.contains(&format!(
            "对白占比：**{:.1}%**",
            dialogue.dialogue_share * 100.0
        )),
        "dialogue share missing from the report"
    );
    assert!(
        markdown.contains("比例只说明对白与叙述的多少"),
        "dialogue section missing its caveat"
    );
    if let Some(sample) = dialogue.samples.first() {
        assert!(
            markdown.contains("### 最长的几处对白（"),
            "dialogue sample list missing from the report"
        );
        assert!(
            markdown.contains(&sample.excerpt),
            "longest speech passage missing from the report"
        );
    } else {
        assert!(
            markdown.contains("未见对白"),
            "text with no detected speech still has to say so"
        );
    }
    // The explicit-language section reports an advisory level and never a grade: a
    // manuscript is not marked down for what its subject matter contains.
    assert!(
        markdown.contains(&format!(
            "内容分级：**{}**（`{}`）",
            match explicit.advisory.as_str() {
                "none" => "词库未见",
                "mild" => "轻微",
                "moderate" => "中等",
                "strong" => "强烈",
                _ => "未知",
            },
            explicit.advisory
        )),
        "explicit advisory missing from the report"
    );
    assert!(
        markdown.contains("本维度不给出评分"),
        "explicit section missing its no-score note"
    );
    if let Some(hit) = explicit.hits.first() {
        assert!(
            markdown.contains("### 命中位置（"),
            "explicit hit list missing from the report"
        );
        assert!(
            markdown.contains(&hit.excerpt),
            "first explicit hit missing from the report"
        );
    } else {
        assert!(
            markdown.contains("没有命中词库里的用词"),
            "an empty explicit pass still has to say so"
        );
    }
    // Story structure describes a shape and never grades one: the section names the
    // landmarks it found, and where the signals cannot place anything it says so rather
    // than filling the window with a guess.
    assert!(
        markdown.contains(&format!(
            "地标 {} 个 · 转折点 {} 个",
            structure.landmarks.len(),
            structure.turn_count
        )),
        "story structure counts missing from the report"
    );
    assert!(
        markdown.contains("本维度不给出评分：契合三幕不是优点"),
        "story structure section missing its no-score note"
    );
    if structure.readable {
        assert!(
            markdown.contains("### 经典模型对照"),
            "model comparison missing from the report"
        );
        assert!(
            markdown.contains("#### 三幕结构：节拍落点"),
            "beat table missing from the report"
        );
    } else {
        assert!(
            markdown.contains("本次不做模型对照"),
            "a document too short for shape matching still has to say why"
        );
    }
    println!(
        "✓ report rendered: {} lines, {} bytes ({} threads, {} pacing segments, {} conflict hits, {} themes, {} scenes, {} cliché hits, {} speech passages, {} explicit hits, {} landmarks, {} recommendations)",
        markdown.lines().count(),
        markdown.len(),
        plot.thread_count,
        pacing.segment_count,
        conflict.total_hits,
        theme.themes.len(),
        setting.scene_count,
        cliche.total_hits,
        dialogue.unit_count,
        explicit.total_hits,
        structure.landmarks.len(),
        assessment.recommendations.len()
    );

    // Persisted and read back: an export writes what was stored, not whatever a
    // re-render at export time would produce.
    let report = StoredReport {
        report_id: Uuid::new_v4(),
        project_id: project.project_id,
        title: Some(format!("{} 分析报告", project.name)),
        markdown: markdown.clone(),
        exported_path: None,
        created_at: generated_at,
    };
    save_report(&conn, &report).expect("save report");
    let stored = get_report(&conn, report.report_id)
        .unwrap()
        .expect("stored report");
    assert_eq!(stored.markdown, markdown);
    assert_eq!(stored.created_at, generated_at);

    // Export: write the stored Markdown to a file and record where it went.
    let path = std::env::temp_dir().join(format!("narrativestudio_report_{}.md", report.report_id));
    std::fs::write(&path, &stored.markdown).expect("write report file");
    mark_exported(&conn, report.report_id, path.to_str().unwrap()).expect("mark exported");

    let on_disk = std::fs::read_to_string(&path).expect("read report file");
    assert_eq!(on_disk, markdown, "file must match the stored report");
    let after = get_report(&conn, report.report_id).unwrap().unwrap();
    assert_eq!(after.exported_path.as_deref(), path.to_str());
    std::fs::remove_file(&path).ok();
    println!("✓ report exported to {} and path recorded", path.display());
}
