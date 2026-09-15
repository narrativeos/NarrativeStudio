//! Validate the single-project analysis pipeline against a real TraceView
//! project folder (default: /Users/futuremeng/.TraceView/出版学基础正文_2).
//!
//! Usage: cargo run -p studio-app --example analysis_check [project_folder]

use std::time::Instant;

use studio_analysis::{run_assessment, run_t0_analysis, run_t1_analysis};
use studio_import::parse_semantic_result_enriched;

fn main() {
    let project_dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/Users/futuremeng/.TraceView/出版学基础正文_2".to_string());

    let project_dir = std::path::Path::new(&project_dir);
    let semantic = std::fs::read_to_string(project_dir.join("semantic/semantic_result.json"))
        .expect("semantic_result.json not found");
    let popo = std::fs::read_to_string(project_dir.join("popo/popo_result.json")).ok();
    let term = std::fs::read_to_string(project_dir.join("semantic/term_result.json")).ok();

    let t = Instant::now();
    let doc = parse_semantic_result_enriched(&semantic, popo.as_deref(), term.as_deref())
        .expect("parse failed");
    println!(
        "parse: {} blocks in {:.2}s",
        doc.blocks.len(),
        t.elapsed().as_secs_f64()
    );

    let t = Instant::now();
    let t0 = run_t0_analysis(&doc).expect("t0 failed");
    let t1 = run_t1_analysis(&doc).expect("t1 failed");
    let assessment = run_assessment(&t0, &t1);
    println!(
        "analyze: T0+T1+assessment in {:.2}s",
        t.elapsed().as_secs_f64()
    );

    println!("\n== T0 ==");
    println!(
        "words={} chars={} sentences={}",
        t0.word_count, t0.char_count, t0.sentence_count
    );
    println!("avg_sentence_chars={:.1}", t0.avg_sentence_chars);
    println!("sentence_dist={:?}", t0.sentence_length_distribution);
    println!(
        "readability: score={:.1} level={} method={}",
        t0.readability.score, t0.readability.level, t0.readability.method
    );
    println!(
        "adverbs={} adjectives={}",
        t0.adverb_count, t0.adjective_count
    );
    println!(
        "top_adverbs={:?}",
        t0.top_adverbs.iter().take(5).collect::<Vec<_>>()
    );
    println!(
        "top_adjectives={:?}",
        t0.top_adjectives.iter().take(5).collect::<Vec<_>>()
    );
    println!(
        "top_repeated_phrases={:?}",
        t0.top_repeated_phrases.iter().take(8).collect::<Vec<_>>()
    );

    println!("\n== T1 ==");
    println!("sections={}", t1.section_count);
    println!(
        "characters={:?}",
        t1.characters
            .iter()
            .take(5)
            .map(|c| (c.name.clone(), c.mentions))
            .collect::<Vec<_>>()
    );
    println!(
        "top_cooccurrences={:?}",
        t1.entity_cooccurrences.iter().take(5).collect::<Vec<_>>()
    );
    println!("arc shape={} points={}", t1.arc.shape, t1.arc.points.len());
    for p in t1.arc.points.iter().take(20) {
        println!(
            "  {:<40} intensity={:.2} chars={}",
            p.section, p.intensity, p.char_count
        );
    }

    println!("\n== Assessment ==");
    println!("overall={:.1}", assessment.overall);
    for d in &assessment.dimensions {
        println!("  {:<16} score={:.1}  {}", d.label, d.score, d.detail);
    }
    println!("recommendations:");
    for r in &assessment.recommendations {
        println!("  [{:<6}] {} — {}", r.severity, r.title, r.detail);
    }
}
