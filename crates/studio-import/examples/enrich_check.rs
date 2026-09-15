//! One-off validation: run the enriched import pipeline on a real TraceView
//! project folder and print the resulting section/noun-signal statistics.

use std::collections::HashMap;

use studio_import::parse_semantic_result_enriched;

fn main() {
    let project_dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/Users/futuremeng/.TraceView/出版学基础正文_2".to_string());
    let project_dir = std::path::Path::new(&project_dir);

    let semantic = std::fs::read_to_string(project_dir.join("semantic/semantic_result.json"))
        .expect("read semantic_result.json");
    let popo_path = project_dir.join("popo/popo_result.json");
    let popo = popo_path
        .exists()
        .then(|| std::fs::read_to_string(&popo_path).expect("read popo_result.json"));
    let term_path = project_dir.join("semantic/term_result.json");
    let terms = term_path
        .exists()
        .then(|| std::fs::read_to_string(&term_path).expect("read term_result.json"));

    let doc = parse_semantic_result_enriched(&semantic, popo.as_deref(), terms.as_deref())
        .expect("parse failed");

    // Section grouping (same logic as T1).
    let mut sections: HashMap<String, (u32, u64)> = HashMap::new();
    for b in &doc.blocks {
        let e = sections.entry(b.section_path.clone()).or_insert((0, 0));
        e.0 += 1;
        e.1 += b.content.len() as u64;
    }
    let mut section_list: Vec<_> = sections.into_iter().collect();
    section_list.sort_by(|a, b| b.1.cmp(&a.1));
    println!("blocks: {}", doc.blocks.len());
    println!("sections: {}", section_list.len());
    for (path, (blocks, chars)) in section_list.iter().take(25) {
        println!("  [{blocks} blocks, {chars} chars] {path}");
    }

    // Noun signal stats (same logic as T0).
    let mut freq: HashMap<String, u32> = HashMap::new();
    let mut total = 0u32;
    for b in &doc.blocks {
        for s in &b.noun_signals {
            *freq.entry(s.text.clone()).or_insert(0) += 1;
            total += 1;
        }
    }
    let mut top: Vec<_> = freq.into_iter().collect();
    top.sort_by(|a, b| b.1.cmp(&a.1));
    println!("noun_signal_count: {total}");
    println!("top noun signals:");
    for (text, count) in top.iter().take(15) {
        println!("  {count:>5}  {text}");
    }
}
