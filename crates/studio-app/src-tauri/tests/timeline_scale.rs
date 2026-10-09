//! Scale check for the time line: what the calendar scan costs on a real book, and
//! which 年号 readings it produces there.
//!
//! Ignored by default — they need real TraceView exports, not the committed fixture:
//!
//! ```bash
//! NARRATIVE_TEST_FILE=~/.TraceView/<book>/semantic/semantic_result.json \
//!   cargo test -p studio-app --test timeline_scale -- --ignored --nocapture
//! NARRATIVE_TEST_DIR=~/.TraceView \
//!   cargo test -p studio-app --test timeline_scale -- --ignored --nocapture
//! ```
//!
//! The budget is the T1 one from the design doc and it covers the analysis pass only:
//! parsing a 100 MB JSON is the importer's problem, not the time line's.

use std::time::Instant;

use studio_analysis::{run_timeline_analysis, TimelineMarker, TimelineStats};
use studio_import::parse_semantic_result;

/// The T1 budget for a whole dimension, per the design doc.
const BUDGET_MS: u128 = 2000;
/// How many calendar readings to print for eyeballing.
const SHOW: usize = 40;

/// The markers that came from a word table rather than from a written number.
fn calendar_forms(t: &TimelineStats) -> Vec<&TimelineMarker> {
    t.markers
        .iter()
        .filter(|m| {
            m.precision.starts_with("era_")
                || m.precision.starts_with("ganzhi_")
                || m.precision.starts_with("lunar_")
                || matches!(m.precision.as_str(), "solar_term" | "festival" | "weekday")
        })
        .collect()
}

#[test]
#[ignore = "needs a real TraceView export in NARRATIVE_TEST_FILE"]
fn the_time_line_stays_inside_the_t1_budget_on_a_real_book() {
    let Ok(path) = std::env::var("NARRATIVE_TEST_FILE") else {
        println!("skipped: set NARRATIVE_TEST_FILE to a real semantic_result.json");
        return;
    };
    let content = std::fs::read_to_string(&path).expect("could not read the file");

    let at = Instant::now();
    let doc = parse_semantic_result(&content).expect("could not parse the semantic result");
    let parse_ms = at.elapsed().as_millis();

    let at = Instant::now();
    let t = run_timeline_analysis(&doc).expect("time-line analysis failed");
    let ms = at.elapsed().as_millis();

    println!("file: {path}");
    println!(
        "blocks {} · chars {} · parse {} ms · timeline {} ms",
        doc.blocks.len(),
        doc.total_char_count,
        parse_ms,
        ms
    );
    println!(
        "absolute {} · partial {} · relative {} · vague {} · entity dates {} · segments {}",
        t.absolute_count,
        t.partial_count,
        t.relative_count,
        t.vague_count,
        t.entity_date_count,
        t.segment_count
    );
    println!("forms: {:?}", t.forms);
    println!(
        "issues: {:?}",
        t.issues.iter().map(|i| &i.kind).collect::<Vec<_>>()
    );
    let hits = calendar_forms(&t);
    println!("calendar-form markers: {}", hits.len());
    for m in hits.iter().take(SHOW) {
        println!(
            "  {} → {} [{}] …{}",
            m.text, m.resolved, m.precision, m.excerpt
        );
    }

    assert!(
        ms < BUDGET_MS,
        "time line took {ms} ms on {} chars, over the {BUDGET_MS} ms T1 budget",
        doc.total_char_count
    );
}

/// Walk a whole TraceView folder and run the time line over every document in it.
///
/// This is the false-hit audit: the era table now holds every dated reign name, several
/// of which are ordinary words (普通, 太平, 长安), so the question is how often a modern
/// book produces an era reading at all. Every calendar reading is printed for a human to
/// judge.
#[test]
#[ignore = "needs a folder of real exports in NARRATIVE_TEST_DIR"]
fn the_time_line_reads_a_whole_folder_without_inventing_dates() {
    let Ok(root) = std::env::var("NARRATIVE_TEST_DIR") else {
        println!("skipped: set NARRATIVE_TEST_DIR to a folder of real exports");
        return;
    };
    let mut files: Vec<String> = Vec::new();
    let mut walk = vec![root];
    while let Some(dir) = walk.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk.push(path.to_string_lossy().to_string());
            } else if path
                .file_name()
                .map(|n| n == "semantic_result.json")
                .unwrap_or(false)
            {
                files.push(path.to_string_lossy().to_string());
            }
        }
    }
    files.sort();
    println!("documents: {}", files.len());

    let mut chars = 0_u64;
    let mut slowest = 0_u128;
    let mut slowest_file = String::new();
    let mut readings = 0_usize;
    for file in &files {
        let Ok(content) = std::fs::read_to_string(file) else {
            println!("  skip (unreadable): {file}");
            continue;
        };
        let Ok(doc) = parse_semantic_result(&content) else {
            println!("  skip (unparsable): {file}");
            continue;
        };
        let at = Instant::now();
        let Ok(t) = run_timeline_analysis(&doc) else {
            println!("  skip (analysis failed): {file}");
            continue;
        };
        let ms = at.elapsed().as_millis();
        chars += doc.total_char_count;
        if ms > slowest {
            slowest = ms;
            slowest_file = file.clone();
        }
        let hits = calendar_forms(&t);
        readings += hits.len();
        if !hits.is_empty() {
            println!("  {} · chars {} · {} ms", file, doc.total_char_count, ms);
            for m in hits.iter().take(12) {
                println!(
                    "    {} → {} [{}] …{}",
                    m.text, m.resolved, m.precision, m.excerpt
                );
            }
        }
        assert!(
            ms < BUDGET_MS,
            "time line took {ms} ms on {} chars in {file}",
            doc.total_char_count
        );
    }
    println!(
        "scanned {} documents · {} chars · slowest {} ms ({slowest_file}) · calendar readings {readings}",
        files.len(),
        chars,
        slowest
    );
}
