//! T1: Structural analysis — chapter/section hierarchy, narrative arc.

use std::collections::HashMap;

use serde::Serialize;
use studio_core::document::DocumentData;

use crate::Result;

/// Section structure info.
#[derive(Debug, Clone, Serialize)]
pub struct SectionInfo {
    pub path: String,
    pub block_count: u32,
    pub char_count: u64,
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

    Ok(T1Stats {
        sections,
        section_count,
        title_blocks,
        paragraph_blocks,
        list_blocks,
        block_type_distribution,
        longest_section,
        shortest_section,
    })
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
    }
}
