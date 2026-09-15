//! Enrich raw semantic blocks with TOC section paths and domain terms.
//!
//! TraceView's `semantic_result.json` often leaves `section_path` empty and
//! omits `noun_signals` entirely. The real chapter structure lives in
//! `popo/popo_result.json` (TOC) and the domain terms in
//! `semantic/term_result.json`. This module merges both into the raw blocks
//! before they are converted into the internal `SemanticBlock` model.

use std::collections::HashMap;

use crate::popo::TocEntry;
use crate::semantic::{RawBlock, RawNounSignal};
use crate::term::Term;

/// Normalize text for title matching: strip HTML tags (`<sub>`, `<sup>`, ...),
/// whitespace, and common (Chinese/English) punctuation so that a TOC title
/// can be compared against block content.
pub fn normalize_for_match(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if in_tag => {}
            c if c.is_whitespace() => {}
            c if is_match_punct(c) => {}
            c => out.push(c),
        }
    }
    out
}

fn is_match_punct(c: char) -> bool {
    matches!(
        c,
        '，' | '。'
            | '、'
            | '；'
            | '：'
            | '？'
            | '！'
            | '…'
            | '—'
            | '–'
            | '·'
            | '～'
            | '※'
            | '＊'
            | '“'
            | '”'
            | '‘'
            | '’'
            | '（'
            | '）'
            | '《'
            | '》'
            | '【'
            | '】'
            | '〈'
            | '〉'
            | '〔'
            | '〕'
            | ','
            | '.'
            | ';'
            | ':'
            | '?'
            | '!'
            | '-'
            | '_'
            | '('
            | ')'
            | '['
            | ']'
            | '{'
            | '}'
            | '/'
            | '\\'
            | '|'
            | '*'
            | '"'
            | '\''
            | '#'
            | '@'
            | '&'
            | '%'
            | '$'
            | '+'
            | '='
            | '~'
            | '`'
    )
}

/// True when a normalized TOC title matches a normalized block content.
///
/// Accepts exact equality, or containment in either direction when the
/// shorter side has at least 2 characters (handles decorated titles such as
/// `第十七章外国出版简史①` vs TOC `第十七章外国出版简史`).
fn title_matches(title_norm: &str, content_norm: &str) -> bool {
    if title_norm.is_empty() || content_norm.is_empty() {
        return false;
    }
    if title_norm == content_norm {
        return true;
    }
    let (short, long) = if title_norm.len() <= content_norm.len() {
        (title_norm, content_norm)
    } else {
        (content_norm, title_norm)
    };
    short.chars().count() >= 2 && long.contains(short)
}

/// Assign `section_path` to blocks based on the popo TOC.
///
/// Each TOC entry is matched to a semantic block by normalized title text,
/// scanning forward only (document order is preserved). Title-type blocks are
/// preferred so that TOC listing pages are not mistaken for chapter starts;
/// non-title blocks only match on exact normalized equality.
///
/// A level stack built from the matched entries produces hierarchical paths
/// like `第一章 出版学的核心概念 / 第一节 出版物`. Blocks before the first
/// matched entry keep their original `section_path`.
///
/// Returns the number of TOC entries that were matched.
pub fn apply_toc(blocks: &mut [RawBlock], toc: &[TocEntry]) -> usize {
    if blocks.is_empty() || toc.is_empty() {
        return 0;
    }

    let contents: Vec<String> = blocks
        .iter()
        .map(|b| normalize_for_match(&b.content))
        .collect();

    // Match each TOC entry to a block index, scanning forward only.
    let mut matched: Vec<(usize, usize)> = Vec::new(); // (toc index, block index)
    let mut search_from = 0usize;
    for (ti, entry) in toc.iter().enumerate() {
        let title_norm = normalize_for_match(&entry.title);
        if title_norm.chars().count() < 2 {
            continue;
        }
        // Pass 1: title-type blocks, equality or containment.
        let mut found = (search_from..blocks.len()).find(|&bi| {
            blocks[bi].block_type.as_deref() == Some("title")
                && title_matches(&title_norm, &contents[bi])
        });
        // Pass 2: any block, exact equality only (avoids TOC listing lines
        // that contain many titles at once).
        if found.is_none() {
            found = (search_from..blocks.len()).find(|&bi| contents[bi] == title_norm);
        }
        if let Some(bi) = found {
            matched.push((ti, bi));
            search_from = bi + 1;
        }
    }
    if matched.is_empty() {
        return 0;
    }

    // Map block index -> TOC entry for the matched boundaries.
    let mut boundaries: HashMap<usize, &TocEntry> = HashMap::new();
    for (ti, bi) in &matched {
        boundaries.insert(*bi, &toc[*ti]);
    }

    // Walk the blocks maintaining a level stack of (level, title).
    let mut stack: Vec<(u32, &str)> = Vec::new();
    let mut current_path: Vec<String> = Vec::new();
    for (bi, block) in blocks.iter_mut().enumerate() {
        if let Some(entry) = boundaries.get(&bi) {
            while stack
                .last()
                .map(|(lvl, _)| *lvl >= entry.level)
                .unwrap_or(false)
            {
                stack.pop();
            }
            stack.push((entry.level, entry.title.as_str()));
            current_path = stack.iter().map(|(_, t)| t.to_string()).collect();
        }
        if !current_path.is_empty() {
            block.section_path = current_path.join(" / ");
        }
    }

    matched.len()
}

/// Attach domain terms as noun signals on the blocks they occur in.
///
/// Terms reference blocks by integer `block_ids`; the first block carrying a
/// given ID is used. Returns the total number of noun signals added.
pub fn apply_terms(blocks: &mut [RawBlock], terms: &[Term]) -> usize {
    if blocks.is_empty() || terms.is_empty() {
        return 0;
    }

    let mut id_to_idx: HashMap<u64, usize> = HashMap::new();
    for (bi, block) in blocks.iter().enumerate() {
        for id in &block.block_ids {
            id_to_idx.entry(*id).or_insert(bi);
        }
    }

    let mut total = 0usize;
    for term in terms {
        if term.text.is_empty() {
            continue;
        }
        for id in &term.block_ids {
            if let Some(&bi) = id_to_idx.get(id) {
                blocks[bi].noun_signals.push(RawNounSignal {
                    text: term.text.clone(),
                    pos: term.category.clone(),
                    syntactic_role: None,
                    score: term.score,
                    span: Vec::new(),
                    evidence: None,
                });
                total += 1;
            }
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_block(id: u64, content: &str, block_type: &str) -> RawBlock {
        RawBlock {
            source_block_ids: vec![],
            block_ids: vec![id],
            content: content.to_string(),
            section_path: String::new(),
            block_type: Some(block_type.to_string()),
            title: None,
            tokens: vec![],
            entities: vec![],
            noun_signals: vec![],
        }
    }

    #[test]
    fn test_normalize_for_match() {
        assert_eq!(
            normalize_for_match("第一章 出版学的核心概念"),
            "第一章出版学的核心概念"
        );
        assert_eq!(
            normalize_for_match("第十七章 外国出版简史<sup>①</sup>"),
            "第十七章外国出版简史①"
        );
        assert_eq!(
            normalize_for_match("第一章 出版学的核心概念 …… 013"),
            "第一章出版学的核心概念013"
        );
    }

    #[test]
    fn test_apply_toc_basic_hierarchy() {
        let mut blocks = vec![
            raw_block(1, "前言内容", "text"),
            raw_block(2, "第一章 出版学的核心概念", "title"),
            raw_block(3, "正文段落一", "text"),
            raw_block(4, "第一节 出版物", "title"),
            raw_block(5, "正文段落二", "text"),
            raw_block(6, "第二章 出版学基本原理", "title"),
            raw_block(7, "正文段落三", "text"),
        ];
        let toc = vec![
            TocEntry {
                title: "第一章 出版学的核心概念".into(),
                level: 1,
            },
            TocEntry {
                title: "第一节 出版物".into(),
                level: 2,
            },
            TocEntry {
                title: "第二章 出版学基本原理".into(),
                level: 1,
            },
        ];
        let matched = apply_toc(&mut blocks, &toc);
        assert_eq!(matched, 3);
        // Blocks before the first boundary keep their original path.
        assert_eq!(blocks[0].section_path, "");
        assert_eq!(blocks[1].section_path, "第一章 出版学的核心概念");
        assert_eq!(blocks[2].section_path, "第一章 出版学的核心概念");
        assert_eq!(
            blocks[3].section_path,
            "第一章 出版学的核心概念 / 第一节 出版物"
        );
        assert_eq!(
            blocks[4].section_path,
            "第一章 出版学的核心概念 / 第一节 出版物"
        );
        assert_eq!(blocks[5].section_path, "第二章 出版学基本原理");
        assert_eq!(blocks[6].section_path, "第二章 出版学基本原理");
    }

    #[test]
    fn test_apply_toc_prefers_title_blocks() {
        // A TOC listing page (text block containing many titles) must not be
        // matched when a real title block exists later in the document.
        let mut blocks = vec![
            raw_block(
                1,
                "第一章 出版学的核心概念 …… 013 第一节 出版物 …… 013",
                "text",
            ),
            raw_block(2, "第一章 出版学的核心概念", "title"),
            raw_block(3, "正文", "text"),
        ];
        let toc = vec![TocEntry {
            title: "第一章 出版学的核心概念".into(),
            level: 1,
        }];
        let matched = apply_toc(&mut blocks, &toc);
        assert_eq!(matched, 1);
        assert_eq!(blocks[0].section_path, "");
        assert_eq!(blocks[1].section_path, "第一章 出版学的核心概念");
        assert_eq!(blocks[2].section_path, "第一章 出版学的核心概念");
    }
    #[test]
    fn test_apply_toc_decorated_title() {
        let mut blocks = vec![
            raw_block(1, "第十七章 外国出版简史<sup>①</sup>", "title"),
            raw_block(2, "正文", "text"),
        ];
        let toc = vec![TocEntry {
            title: "第十七章 外国出版简史".into(),
            level: 1,
        }];
        let matched = apply_toc(&mut blocks, &toc);
        assert_eq!(matched, 1);
        assert_eq!(blocks[1].section_path, "第十七章 外国出版简史");
    }

    #[test]
    fn test_apply_toc_level_jump_and_unmatched() {
        // A level 2 entry without a preceding level 1 match, plus an entry
        // that matches nothing at all.
        let mut blocks = vec![
            raw_block(1, "第一节 出版功能", "title"),
            raw_block(2, "正文", "text"),
            raw_block(3, "第三章 出版学科基础理论", "title"),
            raw_block(4, "正文", "text"),
        ];
        let toc = vec![
            TocEntry {
                title: "第一节 出版功能".into(),
                level: 2,
            },
            TocEntry {
                title: "第十二章 发行".into(),
                level: 1,
            },
            TocEntry {
                title: "第三章 出版学科基础理论".into(),
                level: 1,
            },
        ];
        let matched = apply_toc(&mut blocks, &toc);
        assert_eq!(matched, 2);
        assert_eq!(blocks[0].section_path, "第一节 出版功能");
        assert_eq!(blocks[1].section_path, "第一节 出版功能");
        assert_eq!(blocks[2].section_path, "第三章 出版学科基础理论");
        assert_eq!(blocks[3].section_path, "第三章 出版学科基础理论");
    }

    #[test]
    fn test_apply_toc_no_match_keeps_original() {
        let mut blocks = vec![raw_block(1, "正文", "text")];
        let toc = vec![TocEntry {
            title: "不存在的章节".into(),
            level: 1,
        }];
        let matched = apply_toc(&mut blocks, &toc);
        assert_eq!(matched, 0);
        assert_eq!(blocks[0].section_path, "");
    }

    #[test]
    fn test_apply_toc_empty_inputs() {
        let mut blocks = vec![raw_block(1, "正文", "text")];
        assert_eq!(apply_toc(&mut blocks, &[]), 0);
        let toc = vec![TocEntry {
            title: "第一章".into(),
            level: 1,
        }];
        let mut empty: Vec<RawBlock> = vec![];
        assert_eq!(apply_toc(&mut empty, &toc), 0);
    }

    #[test]
    fn test_apply_terms() {
        let mut blocks = vec![
            raw_block(1, "方卿 许洁 等 编著", "text"),
            raw_block(3, "武汉大学出版社", "text"),
            raw_block(57, "正文段落", "text"),
        ];
        let terms = vec![
            Term {
                text: "方卿".into(),
                category: "PERSON".into(),
                score: 0.17,
                block_ids: vec![1, 57],
            },
            Term {
                text: "武汉大学出版社".into(),
                category: "ORGANIZATION".into(),
                score: 0.19,
                block_ids: vec![3],
            },
            Term {
                text: "不存在的块".into(),
                category: "UNKNOWN".into(),
                score: 0.1,
                block_ids: vec![999],
            },
        ];
        let total = apply_terms(&mut blocks, &terms);
        assert_eq!(total, 3);
        assert_eq!(blocks[0].noun_signals.len(), 1);
        assert_eq!(blocks[0].noun_signals[0].text, "方卿");
        assert_eq!(blocks[0].noun_signals[0].pos, "PERSON");
        assert_eq!(blocks[1].noun_signals.len(), 1);
        assert_eq!(blocks[1].noun_signals[0].text, "武汉大学出版社");
        assert_eq!(blocks[2].noun_signals.len(), 1);
        assert_eq!(blocks[2].noun_signals[0].text, "方卿");
    }

    #[test]
    fn test_apply_terms_empty_inputs() {
        let mut blocks = vec![raw_block(1, "正文", "text")];
        assert_eq!(apply_terms(&mut blocks, &[]), 0);
        let terms = vec![Term {
            text: "术语".into(),
            category: "UNKNOWN".into(),
            score: 0.5,
            block_ids: vec![1],
        }];
        let mut empty: Vec<RawBlock> = vec![];
        assert_eq!(apply_terms(&mut empty, &terms), 0);
    }
}
