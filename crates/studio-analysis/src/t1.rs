//! T1: Structural analysis — chapter/section hierarchy, characters, narrative arc.

use std::collections::HashMap;

use serde::Serialize;
use studio_core::document::DocumentData;
use studio_core::entity::EntityCategory;

use crate::text::{count_quoted_chars, split_sentences, visible_char_count};
use crate::Result;

/// Section structure info.
#[derive(Debug, Clone, Serialize)]
pub struct SectionInfo {
    pub path: String,
    pub block_count: u32,
    pub char_count: u64,
}

/// A character (PERSON entity) profile.
#[derive(Debug, Clone, Serialize)]
pub struct CharacterProfile {
    pub name: String,
    pub mentions: u32,
    pub first_section: String,
    pub last_section: String,
    /// Fraction of the document spanned by the character's appearances (0-1).
    pub span_ratio: f64,
    /// Top co-occurring characters (same block), as (name, count).
    pub top_cooccurrences: Vec<(String, u32)>,
}

/// One point on the narrative arc curve.
#[derive(Debug, Clone, Serialize)]
pub struct ArcPoint {
    pub section: String,
    /// Normalized intensity 0-1.
    pub intensity: f64,
    pub char_count: u64,
}

/// Narrative arc: per-section intensity curve plus a coarse shape label.
#[derive(Debug, Clone, Serialize)]
pub struct NarrativeArc {
    pub points: Vec<ArcPoint>,
    /// "mountain" | "rising" | "falling" | "steady"
    pub shape: String,
}

/// T1 structural statistics.
#[derive(Debug, Clone, Serialize)]
pub struct T1Stats {
    pub sections: Vec<SectionInfo>,
    pub section_count: u32,
    pub title_blocks: u32,
    pub paragraph_blocks: u32,
    pub list_blocks: u32,
    pub block_type_distribution: Vec<(String, u32)>,
    pub longest_section: Option<String>,
    pub shortest_section: Option<String>,
    /// Top PERSON entity profiles by mention count.
    pub characters: Vec<CharacterProfile>,
    /// Per-section narrative intensity curve.
    pub arc: NarrativeArc,
    /// Top entity co-occurrence pairs ("A ↔ B", count) within the same block.
    pub entity_cooccurrences: Vec<(String, u32)>,
}

/// Run T1 structural analysis on the given document.
pub fn run_t1_analysis(doc: &DocumentData) -> Result<T1Stats> {
    // Group blocks by section_path
    let mut sections_map: HashMap<String, (u32, u64)> = HashMap::new();
    let mut block_type_dist: HashMap<String, u32> = HashMap::new();

    let mut title_blocks = 0u32;
    let mut paragraph_blocks = 0u32;
    let mut list_blocks = 0u32;

    for block in &doc.blocks {
        let entry = sections_map
            .entry(block.section_path.clone())
            .or_insert((0, 0));
        entry.0 += 1;
        entry.1 += block.content.len() as u64;

        let bt = block.block_type.as_str();
        *block_type_dist.entry(bt.to_string()).or_insert(0) += 1;

        match bt {
            "title" => title_blocks += 1,
            "paragraph" => paragraph_blocks += 1,
            "list" => list_blocks += 1,
            _ => {}
        }
    }

    let sections: Vec<SectionInfo> = sections_map
        .into_iter()
        .map(|(path, (block_count, char_count))| SectionInfo {
            path,
            block_count,
            char_count,
        })
        .collect();

    let section_count = sections.len() as u32;

    let longest_section = sections
        .iter()
        .max_by_key(|s| s.char_count)
        .map(|s| s.path.clone());
    let shortest_section = sections
        .iter()
        .min_by_key(|s| s.char_count)
        .map(|s| s.path.clone());

    let mut block_type_distribution: Vec<(String, u32)> = block_type_dist.into_iter().collect();
    block_type_distribution.sort_by(|a, b| b.1.cmp(&a.1));

    let characters = analyze_characters(doc);
    let arc = analyze_arc(doc);
    let entity_cooccurrences = analyze_entity_cooccurrences(doc);

    Ok(T1Stats {
        sections,
        section_count,
        title_blocks,
        paragraph_blocks,
        list_blocks,
        block_type_distribution,
        longest_section,
        shortest_section,
        characters,
        arc,
        entity_cooccurrences,
    })
}

/// Top-level section: the first component of a " / "-joined section path.
fn top_level_section(path: &str) -> String {
    if path.is_empty() {
        "(root)".to_string()
    } else {
        path.split(" / ").next().unwrap_or(path).to_string()
    }
}

/// Build character (PERSON entity) profiles: mention counts, first/last
/// appearance, document span, and same-block co-occurrences.
fn analyze_characters(doc: &DocumentData) -> Vec<CharacterProfile> {
    let total_blocks = doc.blocks.len().max(1) as f64;
    let mut mentions: HashMap<String, u32> = HashMap::new();
    let mut first: HashMap<String, (usize, String)> = HashMap::new();
    let mut last: HashMap<String, (usize, String)> = HashMap::new();
    let mut cooccur: HashMap<(String, String), u32> = HashMap::new();

    for (idx, block) in doc.blocks.iter().enumerate() {
        let mut persons: Vec<&str> = block
            .entities
            .iter()
            .filter(|e| e.category == EntityCategory::Person)
            .map(|e| e.text.as_str())
            .collect();
        persons.sort();
        persons.dedup();
        for name in &persons {
            *mentions.entry((*name).to_string()).or_insert(0) += 1;
            first
                .entry((*name).to_string())
                .or_insert((idx, block.section_path.clone()));
            last.insert((*name).to_string(), (idx, block.section_path.clone()));
        }
        for i in 0..persons.len() {
            for j in (i + 1)..persons.len() {
                let (a, b) = (persons[i].to_string(), persons[j].to_string());
                *cooccur.entry((a, b)).or_insert(0) += 1;
            }
        }
    }

    let mut profiles: Vec<CharacterProfile> = mentions
        .into_iter()
        .map(|(name, m)| {
            let (first_idx, first_section) = first.remove(&name).unwrap_or((0, String::new()));
            let (last_idx, last_section) = last.remove(&name).unwrap_or((0, String::new()));
            let span_ratio = (last_idx - first_idx + 1) as f64 / total_blocks;
            let mut partners: Vec<(String, u32)> = cooccur
                .iter()
                .filter(|((a, b), _)| a == &name || b == &name)
                .map(|((a, b), c)| {
                    let other = if a == &name { b.clone() } else { a.clone() };
                    (other, *c)
                })
                .collect();
            partners.sort_by(|x, y| y.1.cmp(&x.1).then_with(|| x.0.cmp(&y.0)));
            partners.truncate(5);
            CharacterProfile {
                name,
                mentions: m,
                first_section,
                last_section,
                span_ratio,
                top_cooccurrences: partners,
            }
        })
        .collect();
    profiles.sort_by(|a, b| {
        b.mentions
            .cmp(&a.mentions)
            .then_with(|| a.name.cmp(&b.name))
    });
    profiles.truncate(20);
    profiles
}

/// Entity categories that carry narrative meaning (excludes noisy DATE/NUMBER).
const PLOT_ENTITY_CATEGORIES: &[EntityCategory] = &[
    EntityCategory::Person,
    EntityCategory::Organization,
    EntityCategory::Location,
    EntityCategory::Facility,
    EntityCategory::Product,
];

/// Top entity co-occurrence pairs within the same block (plot skeleton).
fn analyze_entity_cooccurrences(doc: &DocumentData) -> Vec<(String, u32)> {
    let mut pairs: HashMap<(String, String), u32> = HashMap::new();
    for block in &doc.blocks {
        let mut names: Vec<String> = block
            .entities
            .iter()
            .filter(|e| PLOT_ENTITY_CATEGORIES.contains(&e.category))
            .map(|e| e.text.clone())
            .collect();
        names.sort();
        names.dedup();
        for i in 0..names.len() {
            for j in (i + 1)..names.len() {
                *pairs
                    .entry((names[i].clone(), names[j].clone()))
                    .or_insert(0) += 1;
            }
        }
    }
    let mut out: Vec<(String, u32)> = pairs
        .into_iter()
        .map(|((a, b), c)| (format!("{a} ↔ {b}"), c))
        .collect();
    out.sort_by(|x, y| y.1.cmp(&x.1).then_with(|| x.0.cmp(&y.0)));
    out.truncate(30);
    out
}

/// Raw per-segment counters used to compute narrative intensity.
#[derive(Clone, Default)]
struct SegmentRaw {
    quoted_chars: u64,
    total_chars: u64,
    excl_count: u64,
    short_sentences: u64,
    total_sentences: u64,
    entity_count: u64,
}

/// Build the per-section narrative intensity curve.
///
/// Segments are top-level sections (when there are at least 3); otherwise the
/// document is split into 10 equal block chunks. Intensity combines dialogue
/// ratio, exclamative density, short-sentence ratio, and entity density, each
/// min-max normalized across segments.
fn analyze_arc(doc: &DocumentData) -> NarrativeArc {
    if doc.blocks.is_empty() {
        return NarrativeArc {
            points: vec![],
            shape: "steady".into(),
        };
    }

    let mut order: Vec<String> = Vec::new();
    let mut index_of: HashMap<String, usize> = HashMap::new();
    for block in &doc.blocks {
        let top = top_level_section(&block.section_path);
        if !index_of.contains_key(&top) {
            index_of.insert(top.clone(), order.len());
            order.push(top);
        }
    }

    let use_sections = order.len() >= 3;
    let segment_count = if use_sections { order.len() } else { 10 };
    let chunk_size = (doc.blocks.len() as f64 / segment_count as f64).ceil() as usize;

    let mut raw: Vec<SegmentRaw> = vec![SegmentRaw::default(); segment_count];
    for (idx, block) in doc.blocks.iter().enumerate() {
        let seg = if use_sections {
            *index_of
                .get(&top_level_section(&block.section_path))
                .expect("section was registered above")
        } else {
            (idx / chunk_size.max(1)).min(segment_count - 1)
        };
        let r = &mut raw[seg];
        r.total_chars += visible_char_count(&block.content) as u64;
        r.quoted_chars += count_quoted_chars(&block.content) as u64;
        r.excl_count += block
            .content
            .chars()
            .filter(|c| matches!(c, '！' | '!' | '？' | '?'))
            .count() as u64;
        for sentence in split_sentences(&block.content) {
            let len = visible_char_count(&sentence);
            if len > 0 {
                r.total_sentences += 1;
                if len <= 12 {
                    r.short_sentences += 1;
                }
            }
        }
        r.entity_count += block.entities.len() as u64;
    }

    let dialogue: Vec<f64> = raw
        .iter()
        .map(|r| {
            if r.total_chars > 0 {
                r.quoted_chars as f64 / r.total_chars as f64
            } else {
                0.0
            }
        })
        .collect();
    let excl: Vec<f64> = raw
        .iter()
        .map(|r| {
            if r.total_chars > 0 {
                r.excl_count as f64 / r.total_chars as f64 * 1000.0
            } else {
                0.0
            }
        })
        .collect();
    let short: Vec<f64> = raw
        .iter()
        .map(|r| {
            if r.total_sentences > 0 {
                r.short_sentences as f64 / r.total_sentences as f64
            } else {
                0.0
            }
        })
        .collect();
    let entity: Vec<f64> = raw
        .iter()
        .map(|r| {
            if r.total_chars > 0 {
                r.entity_count as f64 / r.total_chars as f64 * 1000.0
            } else {
                0.0
            }
        })
        .collect();

    let dialogue = normalize(&dialogue);
    let excl = normalize(&excl);
    let short = normalize(&short);
    let entity = normalize(&entity);

    let points: Vec<ArcPoint> = raw
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let intensity =
                0.35 * dialogue[i] + 0.25 * excl[i] + 0.20 * short[i] + 0.20 * entity[i];
            let section = if use_sections {
                order[i].clone()
            } else {
                format!("{}/{}", i + 1, segment_count)
            };
            ArcPoint {
                section,
                intensity,
                char_count: r.total_chars,
            }
        })
        .collect();

    NarrativeArc {
        shape: classify_shape(&points),
        points,
    }
}

/// Min-max normalize a slice of values to 0-1 (all zeros when no variance).
fn normalize(values: &[f64]) -> Vec<f64> {
    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    if (max - min).abs() < f64::EPSILON {
        return vec![0.0; values.len()];
    }
    values.iter().map(|v| (v - min) / (max - min)).collect()
}

/// Coarse shape classification from the first/middle/last thirds of the curve.
fn classify_shape(points: &[ArcPoint]) -> String {
    if points.len() < 3 {
        return "steady".to_string();
    }
    let third = (points.len() / 3).max(1);
    let avg = |slice: &[ArcPoint]| {
        slice.iter().map(|p| p.intensity).sum::<f64>() / slice.len().max(1) as f64
    };
    let a = avg(&points[..third]);
    let b = avg(&points[third..points.len() - third]);
    let c = avg(&points[points.len() - third..]);
    if b > a + 0.05 && b > c + 0.05 {
        "mountain".to_string()
    } else if c > a + 0.05 && c >= b - 0.05 {
        "rising".to_string()
    } else if a > c + 0.05 && a >= b - 0.05 {
        "falling".to_string()
    } else {
        "steady".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::document::SemanticBlock;

    fn block(path: &str, bt: &str, content: &str) -> SemanticBlock {
        SemanticBlock {
            source_block_id: "blk-test".into(),
            content: content.to_string(),
            section_path: path.to_string(),
            block_type: bt.to_string(),
            title: None,
            tokens: vec![],
            entities: vec![],
            noun_signals: vec![],
        }
    }

    fn sample_doc() -> DocumentData {
        DocumentData {
            doc_id: uuid::Uuid::new_v4(),
            title: "Test".into(),
            blocks: vec![
                block("Ch1", "title", "Chapter One"),
                block("Ch1", "paragraph", "This is a long paragraph with content."),
                block("Ch1", "paragraph", "Another paragraph here."),
                block("Ch2", "title", "Chapter Two"),
                block("Ch2", "list", "- item one\n- item two"),
            ],
            total_word_count: 10,
            total_char_count: 100,
        }
    }

    #[test]
    fn test_t1_section_count() {
        let doc = sample_doc();
        let stats = run_t1_analysis(&doc).unwrap();
        assert_eq!(stats.section_count, 2);
    }

    #[test]
    fn test_t1_block_types() {
        let doc = sample_doc();
        let stats = run_t1_analysis(&doc).unwrap();
        assert_eq!(stats.title_blocks, 2);
        assert_eq!(stats.paragraph_blocks, 2);
        assert_eq!(stats.list_blocks, 1);
    }

    #[test]
    fn test_t1_longest_section() {
        let doc = sample_doc();
        let stats = run_t1_analysis(&doc).unwrap();
        // Ch1 has more chars (2 paragraphs vs 1 list)
        assert_eq!(stats.longest_section.as_deref(), Some("Ch1"));
    }

    #[test]
    fn test_t1_empty_doc() {
        let doc = DocumentData {
            doc_id: uuid::Uuid::new_v4(),
            title: "Empty".into(),
            blocks: vec![],
            total_word_count: 0,
            total_char_count: 0,
        };
        let stats = run_t1_analysis(&doc).unwrap();
        assert_eq!(stats.section_count, 0);
        assert!(stats.longest_section.is_none());
        assert!(stats.characters.is_empty());
        assert!(stats.arc.points.is_empty());
    }

    fn person_block(path: &str, persons: &[&str]) -> SemanticBlock {
        let entities = persons
            .iter()
            .enumerate()
            .map(|(i, name)| studio_core::entity::Entity {
                text: (*name).to_string(),
                category: studio_core::entity::EntityCategory::Person,
                confidence: 0.9,
                source: "test".into(),
                keep: true,
                filter: None,
                filter_reason: None,
                span: (i * 2, i * 2 + 2),
            })
            .collect();
        SemanticBlock {
            source_block_id: format!("blk-{}", path.replace(' ', "")),
            content: "内容".to_string(),
            section_path: path.to_string(),
            block_type: "paragraph".to_string(),
            title: None,
            tokens: vec![],
            entities,
            noun_signals: vec![],
        }
    }

    #[test]
    fn test_t1_characters() {
        let doc = DocumentData {
            doc_id: uuid::Uuid::new_v4(),
            title: "C".into(),
            blocks: vec![
                person_block("Ch1", &["Alice", "Bob"]),
                person_block("Ch2", &["Alice"]),
                person_block("Ch3", &["Alice", "Bob"]),
            ],
            total_word_count: 3,
            total_char_count: 6,
        };
        let stats = run_t1_analysis(&doc).unwrap();
        assert_eq!(stats.characters.len(), 2);
        let alice = &stats.characters[0];
        assert_eq!(alice.name, "Alice");
        assert_eq!(alice.mentions, 3);
        assert_eq!(alice.first_section, "Ch1");
        assert_eq!(alice.last_section, "Ch3");
        assert!((alice.span_ratio - 1.0).abs() < f64::EPSILON);
        assert_eq!(alice.top_cooccurrences[0], ("Bob".to_string(), 2));
        let bob = &stats.characters[1];
        assert_eq!(bob.mentions, 2);
        // Bob appears in Ch1 (idx 0) and Ch3 (idx 2) => spans all 3 blocks.
        assert!((bob.span_ratio - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_t1_entity_cooccurrences() {
        let doc = DocumentData {
            doc_id: uuid::Uuid::new_v4(),
            title: "E".into(),
            blocks: vec![
                person_block("Ch1", &["Alice", "Bob"]),
                person_block("Ch2", &["Alice", "Bob"]),
            ],
            total_word_count: 2,
            total_char_count: 4,
        };
        let stats = run_t1_analysis(&doc).unwrap();
        assert_eq!(stats.entity_cooccurrences.len(), 1);
        assert_eq!(stats.entity_cooccurrences[0].0, "Alice ↔ Bob");
        assert_eq!(stats.entity_cooccurrences[0].1, 2);
    }

    #[test]
    fn test_t1_arc_uses_sections() {
        let doc = DocumentData {
            doc_id: uuid::Uuid::new_v4(),
            title: "A".into(),
            blocks: vec![
                person_block("Ch1", &[]),
                person_block("Ch2", &[]),
                person_block("Ch3", &[]),
            ],
            total_word_count: 3,
            total_char_count: 6,
        };
        let stats = run_t1_analysis(&doc).unwrap();
        assert_eq!(stats.arc.points.len(), 3);
        assert_eq!(stats.arc.points[0].section, "Ch1");
        // No variance in any metric => all intensities 0, shape steady.
        assert!(stats.arc.points.iter().all(|p| p.intensity == 0.0));
        assert_eq!(stats.arc.shape, "steady");
    }

    #[test]
    fn test_t1_arc_falls_back_to_chunks() {
        // Fewer than 3 sections => 10 equal chunks.
        let doc = DocumentData {
            doc_id: uuid::Uuid::new_v4(),
            title: "A".into(),
            blocks: vec![person_block("Only", &[]), person_block("Only", &[])],
            total_word_count: 2,
            total_char_count: 4,
        };
        let stats = run_t1_analysis(&doc).unwrap();
        assert_eq!(stats.arc.points.len(), 10);
        assert_eq!(stats.arc.points[0].section, "1/10");
    }

    #[test]
    fn test_t1_arc_rising_shape() {
        // Exclamations concentrated in the last section => rising curve.
        let mut blocks = vec![person_block("Ch1", &[]), person_block("Ch2", &[])];
        let mut last = person_block("Ch3", &[]);
        last.content = "太好了！成功了！".to_string();
        blocks.push(last);
        let doc = DocumentData {
            doc_id: uuid::Uuid::new_v4(),
            title: "A".into(),
            blocks,
            total_word_count: 3,
            total_char_count: 10,
        };
        let stats = run_t1_analysis(&doc).unwrap();
        assert_eq!(stats.arc.shape, "rising");
        assert!(stats.arc.points[2].intensity > stats.arc.points[0].intensity);
    }

    #[test]
    fn test_classify_shape_mountain() {
        let points = vec![
            ArcPoint {
                section: "a".into(),
                intensity: 0.1,
                char_count: 0,
            },
            ArcPoint {
                section: "b".into(),
                intensity: 0.9,
                char_count: 0,
            },
            ArcPoint {
                section: "c".into(),
                intensity: 0.2,
                char_count: 0,
            },
        ];
        assert_eq!(classify_shape(&points), "mountain");
    }
}
