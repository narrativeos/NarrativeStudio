//! Shared curve segmentation for the section-by-section analyses.
//!
//! Pacing, conflict and setting all draw a curve over the document, and a curve
//! is only comparable if its X axis is cut the same way: three or more top-level
//! sections use those sections, anything less is cut into `FALLBACK_SEGMENTS`
//! equal block chunks. Keeping the cut in one place is what lets the pacing
//! curve, the conflict curve and the setting curve be read against each other
//! (and against the T1 arc, which walks the same sections).
//!
//! Deterministic: segments are in reading order (first appearance of the
//! section), never HashMap order.

use std::collections::HashMap;

use studio_core::document::DocumentData;

use crate::t1::top_level_section;

/// Below this many top-level sections the document is chunked by block instead:
/// with two segments, "flat" and "varied" say the same thing about one long block.
pub(crate) const MIN_SECTIONS: usize = 3;
/// Segments used when the document has no usable section structure.
pub(crate) const FALLBACK_SEGMENTS: usize = 10;

/// How the document is cut into curve segments.
pub(crate) struct SegmentPlan {
    /// Segment labels in reading order.
    pub labels: Vec<String>,
    /// True when the labels are block chunks (`"3/10"`) rather than sections.
    pub chunked: bool,
    /// One entry per block, in `doc.blocks` order: the segment that block falls in.
    pub segment_of: Vec<usize>,
}

/// Cut the document into curve segments.
///
/// Empty `labels` means there is nothing to cut (no blocks).
pub(crate) fn plan_segments(doc: &DocumentData) -> SegmentPlan {
    if doc.blocks.is_empty() {
        return SegmentPlan {
            labels: Vec::new(),
            chunked: false,
            segment_of: Vec::new(),
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

    let use_sections = order.len() >= MIN_SECTIONS;
    let count = if use_sections {
        order.len()
    } else {
        FALLBACK_SEGMENTS
    };

    let labels = if use_sections {
        order
    } else {
        (0..count).map(|i| format!("{}/{}", i + 1, count)).collect()
    };

    let chunk_size = (doc.blocks.len() as f64 / count as f64).ceil() as usize;
    let segment_of = doc
        .blocks
        .iter()
        .enumerate()
        .map(|(idx, block)| {
            if use_sections {
                *index_of
                    .get(&top_level_section(&block.section_path))
                    .expect("section was registered above")
            } else {
                (idx / chunk_size.max(1)).min(count - 1)
            }
        })
        .collect();

    SegmentPlan {
        labels,
        chunked: !use_sections,
        segment_of,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::document::SemanticBlock;

    fn para(path: &str) -> SemanticBlock {
        SemanticBlock {
            source_block_id: "blk-test".into(),
            content: "内容".into(),
            section_path: path.into(),
            block_type: "paragraph".into(),
            title: None,
            tokens: vec![],
            entities: vec![],
            noun_signals: vec![],
        }
    }

    fn doc_of(blocks: Vec<SemanticBlock>) -> DocumentData {
        DocumentData {
            doc_id: uuid::Uuid::nil(),
            title: "Test".into(),
            blocks,
            total_word_count: 0,
            total_char_count: 0,
        }
    }

    #[test]
    fn test_sections_are_used_when_there_are_enough() {
        // Reading order, not HashMap order, and nested paths collapse to their
        // top-level section.
        let plan = plan_segments(&doc_of(vec![
            para("第三章 / 二"),
            para("第一章"),
            para("第一章 / 二"),
            para("第二章"),
        ]));
        assert_eq!(plan.labels, ["第三章", "第一章", "第二章"]);
        assert_eq!(plan.segment_of, [0, 1, 1, 2]);
        assert!(!plan.chunked);
    }

    #[test]
    fn test_two_sections_fall_back_to_chunks() {
        let blocks: Vec<SemanticBlock> = (0..25).map(|_| para("同一个章节")).collect();
        let plan = plan_segments(&doc_of(blocks));

        assert_eq!(plan.labels.len(), FALLBACK_SEGMENTS);
        assert_eq!(plan.labels[2], "3/10");
        assert!(plan.chunked);
        // 25 blocks over 10 segments is 3 blocks per chunk, and the last segment
        // absorbs the remainder instead of overflowing.
        assert_eq!(plan.segment_of[0], 0);
        assert_eq!(plan.segment_of[4], 1);
        assert_eq!(plan.segment_of[24], 8);
        assert!(plan.segment_of.iter().all(|s| *s < plan.labels.len()));
    }

    #[test]
    fn test_empty_document_has_no_segments() {
        let plan = plan_segments(&doc_of(vec![]));
        assert!(plan.labels.is_empty());
        assert!(!plan.chunked);
    }
}
