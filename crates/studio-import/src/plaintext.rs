//! Plain-text import: Markdown and TXT into the same `DocumentData`.
//!
//! TraceView hands the analysis engine four things a `.md` file does not:
//! tokens with part-of-speech tags, entities, noun signals, and a chapter TOC
//! kept in a separate file. This module produces **none of them on purpose**.
//! Guessing word boundaries or inventing entities would fill the plot, theme
//! and setting sections with numbers that look measured and are not — the one
//! failure mode the rest of the engine is built to avoid (see the `readable`
//! gate in `structure.rs` and the `source` field in `theme.rs`). A text-only
//! import therefore carries empty `tokens` / `entities` / `noun_signals`, and
//! the dimensions that need them say they cannot read the text instead of
//! reporting zero of whatever they never looked for.
//!
//! What it does carry is the part every dimension can use: block boundaries and
//! section paths. Headings become `title` blocks and maintain a section stack in
//! the same `" / "` form the popo TOC enrichment produces, so `t1` sees a real
//! chapter structure and scene cutting sees real scene starts.

use studio_core::document::{DocumentData, SemanticBlock};
use studio_core::entity::Entity;
use studio_core::error::Result;
use studio_core::noun_signal::NounSignal;
use uuid::Uuid;

/// How a plain-text file is to be read. The caller picks this from the file
/// extension; the two differ in more than markup, because a `.txt` novel tends
/// to put one paragraph per line while a `.md` file wraps at a column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextFormat {
    Markdown,
    Txt,
}

/// One block under construction: its text, its type, and the section it sits in.
struct Draft {
    content: String,
    block_type: &'static str,
    section_path: String,
    title: Option<String>,
}

/// Parse a Markdown or plain-text document.
///
/// `title` is the document title (the caller uses the file name stem); it is
/// stored on the document and never inserted into the body.
pub fn parse_plain_text(title: &str, text: &str, format: TextFormat) -> Result<DocumentData> {
    let text = strip_html_comments(&strip_front_matter(&normalize_newlines(text)));

    let drafts = match format {
        TextFormat::Markdown => markdown_drafts(&text),
        TextFormat::Txt => txt_drafts(&text),
    };

    let blocks: Vec<SemanticBlock> = drafts
        .into_iter()
        .filter(|d| !d.content.trim().is_empty())
        .map(|d| SemanticBlock {
            // Left empty on purpose: `save_document` issues a fresh id for any
            // block that arrives without one, which keeps two imports of the
            // same file from colliding on the primary key. The analysis cache
            // is keyed per project, so the only stability that matters is within
            // a project, and the id the database assigns is stable there.
            source_block_id: String::new(),
            content: d.content,
            section_path: d.section_path,
            block_type: d.block_type.to_string(),
            title: d.title,
            tokens: vec![],
            entities: Vec::<Entity>::new(),
            noun_signals: Vec::<NounSignal>::new(),
        })
        .collect();

    // Byte length, matching `semantic.rs`: both importers must mean the same
    // thing by this number, or the same book would report a different length
    // depending on which door it came in through.
    let total_char_count = blocks.iter().map(|b| b.content.len() as u64).sum();

    Ok(DocumentData {
        doc_id: Uuid::new_v4(),
        title: title.to_string(),
        blocks,
        // No segmentation, so no word count. Zero here means *not measured*,
        // which `T0Stats::token_source` says out loud wherever it is shown.
        total_word_count: 0,
        total_char_count,
    })
}

/// Convenience wrapper: parse Markdown.
pub fn parse_markdown(title: &str, text: &str) -> Result<DocumentData> {
    parse_plain_text(title, text, TextFormat::Markdown)
}

/// Convenience wrapper: parse plain text.
pub fn parse_txt(title: &str, text: &str) -> Result<DocumentData> {
    parse_plain_text(title, text, TextFormat::Txt)
}

/// Drop a YAML front-matter block: only when the very first line is `---`, so a
/// horizontal rule in the body is never mistaken for one.
fn strip_front_matter(text: &str) -> String {
    if text.lines().next().map(str::trim) != Some("---") {
        return text.to_string();
    }
    let mut rest = String::new();
    let mut closed = false;
    for line in text.lines().skip(1) {
        if closed {
            rest.push_str(line);
            rest.push('\n');
        } else if matches!(line.trim(), "---" | "...") {
            closed = true;
        }
    }
    // An unterminated block means those dashes were not front matter after all.
    if closed {
        rest
    } else {
        text.to_string()
    }
}

fn strip_html_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        match rest[start..].find("-->") {
            Some(end) => rest = &rest[start + end + 3..],
            // Unterminated: keep the rest verbatim rather than deleting prose.
            None => {
                out.push_str(&rest[start..]);
                return out;
            }
        }
    }
    out.push_str(rest);
    out
}

/// The heading stack, rendered in the `" / "` form the TOC enrichment uses.
#[derive(Default)]
struct SectionStack(Vec<(usize, String)>);

impl SectionStack {
    fn enter(&mut self, level: usize, title: &str) -> String {
        self.0.truncate(level.saturating_sub(1));
        self.0.push((level, title.to_string()));
        self.path()
    }

    fn path(&self) -> String {
        self.0
            .iter()
            .map(|(_, t)| t.as_str())
            .collect::<Vec<_>>()
            .join(" / ")
    }
}

/// Join the lines of one paragraph.
///
/// A soft line break is not a sentence break: `split_sentences` ends a sentence
/// at `\n`, so joining wrapped lines with a newline would invent a sentence at
/// every wrap. Chinese prose wraps without a space, English does not, so the
/// separator depends on what the two sides of the break are.
fn join_lines(lines: &[String]) -> String {
    let mut out = String::new();
    for line in lines {
        let piece = line.trim();
        if piece.is_empty() {
            continue;
        }
        if !out.is_empty() {
            let needs_space = matches!(
                (out.chars().next_back(), piece.chars().next()),
                (Some(a), Some(b)) if (a.is_ascii_alphanumeric() && b.is_ascii_alphanumeric())
                    || a.is_ascii_whitespace()
                    || b.is_ascii_whitespace()
            );
            if needs_space {
                out.push(' ');
            }
        }
        out.push_str(piece);
    }
    out
}

/// Normalise `\r\n` and lone `\r` to `\n`, so a file written on Windows or an old
/// Mac splits into lines the same way as one written here.
fn normalize_newlines(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut prev_cr = false;
    for c in text.chars() {
        match c {
            '\r' => {
                if !prev_cr {
                    out.push('\n');
                }
                prev_cr = true;
            }
            '\n' => {
                if !prev_cr {
                    out.push('\n');
                }
                prev_cr = false;
            }
            _ => {
                out.push(c);
                prev_cr = false;
            }
        }
    }
    out
}

/// Split Markdown into blocks.
fn markdown_drafts(text: &str) -> Vec<Draft> {
    let mut drafts: Vec<Draft> = Vec::new();
    let mut stack = SectionStack::default();
    let mut para: Vec<String> = Vec::new();
    let mut fence: Option<String> = None;
    let mut fenced: Vec<String> = Vec::new();

    for raw in text.lines() {
        let line = raw.trim_end();

        if let Some(marker) = fence_marker(line) {
            match fence.take() {
                // Inside a fence every line is kept verbatim: markup stripped
                // out of it would rewrite text the author wrote literally.
                None => fence = Some(marker),
                Some(_) => {
                    flush_paragraph(&mut para, &stack, &mut drafts);
                    let content = join_lines(&fenced);
                    fenced.clear();
                    if !content.trim().is_empty() {
                        drafts.push(Draft {
                            content,
                            block_type: "paragraph",
                            section_path: stack.path(),
                            title: None,
                        });
                    }
                }
            }
            continue;
        }
        if fence.is_some() {
            fenced.push(raw.to_string());
            continue;
        }

        if let Some((level, heading)) = atx_heading(line) {
            flush_paragraph(&mut para, &stack, &mut drafts);
            let path = stack.enter(level, &heading);
            drafts.push(Draft {
                content: heading.clone(),
                block_type: "title",
                section_path: path,
                title: Some(heading),
            });
            continue;
        }

        if let Some(heading) = chapter_heading(line) {
            flush_paragraph(&mut para, &stack, &mut drafts);
            let path = stack.enter(1, &heading);
            drafts.push(Draft {
                content: heading.clone(),
                block_type: "title",
                section_path: path,
                title: Some(heading),
            });
            continue;
        }

        // A thematic break is a scene separator in prose. It is dropped: scene
        // cutting keys on title blocks, and a line of dashes left in the body
        // would be read as a spoken line by the dialogue pass, which treats a
        // leading `——` as dialogue.
        if is_thematic_break(line) {
            flush_paragraph(&mut para, &stack, &mut drafts);
            continue;
        }

        if let Some(item) = list_item(line) {
            flush_paragraph(&mut para, &stack, &mut drafts);
            drafts.push(Draft {
                content: item,
                block_type: "list",
                section_path: stack.path(),
                title: None,
            });
            continue;
        }

        let body = strip_blockquote(line);
        if body.trim().is_empty() {
            flush_paragraph(&mut para, &stack, &mut drafts);
            continue;
        }
        para.push(strip_inline_markup(body.trim()));
    }

    // A fence left open at the end of the file means the closing marker was
    // simply forgotten; the lines are prose as far as the reader is concerned.
    if !fenced.is_empty() {
        let content = join_lines(&fenced);
        if !content.trim().is_empty() {
            drafts.push(Draft {
                content,
                block_type: "paragraph",
                section_path: stack.path(),
                title: None,
            });
        }
    }
    flush_paragraph(&mut para, &stack, &mut drafts);
    drafts
}

/// Split plain text into blocks.
///
/// A `.txt` novel is written one paragraph per line, so a file with no blank
/// lines at all is read line by line; once the author used blank lines, they are
/// the paragraph separators and a lone newline is only a wrap.
fn txt_drafts(text: &str) -> Vec<Draft> {
    let mut drafts: Vec<Draft> = Vec::new();
    let mut stack = SectionStack::default();
    let paragraph_mode = text.lines().any(|l| l.trim().is_empty());
    let mut para: Vec<String> = Vec::new();

    for raw in text.lines() {
        let line = raw.trim();
        if paragraph_mode && line.is_empty() {
            flush_paragraph(&mut para, &stack, &mut drafts);
            continue;
        }
        if let Some(heading) = chapter_heading(line) {
            flush_paragraph(&mut para, &stack, &mut drafts);
            let path = stack.enter(1, &heading);
            drafts.push(Draft {
                content: heading.clone(),
                block_type: "title",
                section_path: path,
                title: Some(heading),
            });
            continue;
        }
        if line.is_empty() || is_thematic_break(line) {
            flush_paragraph(&mut para, &stack, &mut drafts);
            continue;
        }
        // No markup is stripped here. A `*` in a `.txt` novel is a scene break
        // or a footnote mark, not emphasis, and deleting it would be a guess.
        if paragraph_mode {
            para.push(line.to_string());
        } else {
            drafts.push(Draft {
                content: line.to_string(),
                block_type: "paragraph",
                section_path: stack.path(),
                title: None,
            });
        }
    }
    flush_paragraph(&mut para, &stack, &mut drafts);
    drafts
}

/// `# 标题` .. `###### 标题`, with the optional closing run of `#` removed.
///
/// A space after the hashes is not required: `#第三章` is common enough in
/// hand-typed Chinese to be worth accepting, and `#` never begins prose.
fn atx_heading(line: &str) -> Option<(usize, String)> {
    let trimmed = line.trim_start();
    let level = trimmed.chars().take_while(|c| *c == '#').count();
    if level == 0 || level > 6 {
        return None;
    }
    let rest = trimmed[level..].trim();
    let rest = rest.trim_end_matches('#').trim();
    if rest.is_empty() {
        return None;
    }
    Some((level, rest.to_string()))
}

/// A chapter line written without markup: `第三章`, `第三十一章 归途`, `序章`,
/// `尾声`, `第3回`.
///
/// Deliberately narrow. The numeral run must be followed by a section word and
/// the line must carry no sentence punctuation, so `第一天，他推开门` (prose) and
/// `第三章的某个夜晚` (prose) are both left alone — a false heading would move
/// every block after it into the wrong section.
fn chapter_heading(line: &str) -> Option<String> {
    let line = line.trim();
    if line.is_empty() || line.chars().count() > 40 {
        return None;
    }
    if line.contains(['。', '！', '？', '，', '；', '：']) {
        return None;
    }

    const STANDALONE: &[&str] = &[
        "序章", "楔子", "前言", "引言", "导言", "后记", "后叙", "尾声", "终章", "番外",
    ];
    for word in STANDALONE {
        if let Some(rest) = line.strip_prefix(word) {
            // Same rule as below: the word must stand alone or be followed by a
            // separator, so `后记三则` is a title and `前言不搭后语` is prose.
            if rest.is_empty() || starts_with_separator(rest) {
                let rest = rest.trim_start();
                if rest.chars().count() <= 24 {
                    return Some(line.to_string());
                }
            }
        }
    }

    let mut chars = line.chars();
    if chars.next() != Some('第') {
        return None;
    }
    let mut numeral = 0usize;
    let mut section_word = false;
    for c in chars.by_ref() {
        if is_numeral_char(c) {
            numeral += 1;
            continue;
        }
        if numeral == 0 || !matches!(c, '章' | '节' | '回' | '卷' | '篇' | '幕' | '部') {
            return None;
        }
        section_word = true;
        break;
    }
    // A numeral with no section word (`第二`) is a fragment of prose, not a
    // chapter: it is almost always the start of a sentence.
    if numeral == 0 || !section_word {
        return None;
    }
    let rest: String = chars.as_str().to_string();
    // The section word must end the line or be followed by a separator, so
    // `第三章的某个夜晚` stays prose: reading it as a heading would move every
    // block after it into a section that does not exist.
    if !rest.is_empty() && !starts_with_separator(&rest) {
        return None;
    }
    let rest = rest.trim_start_matches([' ', '、', '·', '-', '—']);
    if !rest.is_empty() && rest.chars().count() > 24 {
        return None;
    }
    Some(line.to_string())
}

fn starts_with_separator(s: &str) -> bool {
    s.starts_with([' ', '、', '·', '-', '—', '\u{3000}'])
}

fn is_numeral_char(c: char) -> bool {
    matches!(
        c,
        '零' | '〇'
            | '一'
            | '二'
            | '三'
            | '四'
            | '五'
            | '六'
            | '七'
            | '八'
            | '九'
            | '十'
            | '百'
            | '千'
            | '万'
            | '两'
            | '0'..='9'
    )
}

/// A line that is only a rule: `---`, `***`, `___`, or the Chinese scene break
/// `———`.
///
/// The em-dash form matters most: the dialogue pass reads a line *starting* with
/// `——` as a spoken line, so a separator written in dashes would otherwise be
/// counted as one more piece of dialogue with nothing in it.
fn is_thematic_break(line: &str) -> bool {
    let trimmed = line.trim();
    let is_mark = |c: char| matches!(c, '*' | '-' | '_' | '—' | '─');
    trimmed.chars().filter(|c| is_mark(*c)).count() >= 3
        && trimmed.chars().all(|c| is_mark(c) || c == ' ')
}

fn fence_marker(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let run: String = trimmed
        .chars()
        .take_while(|c| *c == '`' || *c == '~')
        .collect();
    if run.len() >= 3 {
        Some(run)
    } else {
        None
    }
}

fn list_item(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let after_marker = if let Some(rest) = trimmed.strip_prefix(['-', '*', '+']) {
        rest
    } else {
        let digits: String = trimmed.chars().take_while(|c| c.is_ascii_digit()).collect();
        if digits.is_empty() {
            return None;
        }
        trimmed[digits.len()..].strip_prefix(['.', ')', '、'])?
    };
    // The marker must be followed by whitespace or CJK; otherwise `-3.5` and
    // `*如此` (a footnote mark) would be read as list syntax.
    if !after_marker.starts_with(char::is_whitespace) && !after_marker.starts_with(is_cjk_char) {
        return None;
    }
    let item = after_marker.trim();
    if item.is_empty() {
        return None;
    }
    Some(strip_inline_markup(item))
}

fn strip_blockquote(line: &str) -> &str {
    let mut rest = line.trim_start();
    while let Some(after) = rest.strip_prefix('>') {
        rest = after.strip_prefix(' ').unwrap_or(after);
    }
    rest
}

fn is_cjk_char(c: char) -> bool {
    matches!(c as u32, 0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0xF900..=0xFAFF)
}

/// Strip Markdown's inline markup, keeping the text it wraps.
///
/// This is a literal transform, not a Markdown interpreter: emphasis markers
/// that are not paired are left alone, and a `<` that opens nothing is left
/// alone. It runs at import rather than at analysis because the analysis engine
/// counts characters — voice's punctuation density, cliche's literal scan, every
/// 每千字 figure — and `**` left in the body would be counted as prose.
fn strip_inline_markup(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut open_delims: Vec<(char, usize)> = Vec::new();
    let mut i = 0usize;

    let run_len = |from: usize, marker: char| -> usize {
        chars[from..].iter().take_while(|c| **c == marker).count()
    };
    let find_run = |from: usize, marker: char, len: usize| -> Option<usize> {
        let mut j = from;
        while j < chars.len() {
            if chars[j] == marker && run_len(j, marker) == len {
                return Some(j);
            }
            j += 1;
        }
        None
    };

    while i < chars.len() {
        let c = chars[i];
        match c {
            '\\' => {
                // An escape only swallows the next character when that character
                // is punctuation; `\n` in prose stays two characters.
                match chars.get(i + 1) {
                    Some(&next) if next.is_ascii_punctuation() => {
                        out.push(next);
                        i += 2;
                    }
                    _ => {
                        out.push(c);
                        i += 1;
                    }
                }
            }
            '`' => {
                let len = run_len(i, '`');
                match find_run(i + len, '`', len) {
                    Some(close) => {
                        out.extend(&chars[i + len..close]);
                        i = close + len;
                    }
                    None => {
                        out.push(c);
                        i += 1;
                    }
                }
            }
            '!' | '[' => {
                // Images and links keep their text; the target is a path, not
                // prose, and counting it would move every density figure.
                let is_image = c == '!';
                let open = if is_image { i + 1 } else { i };
                if chars.get(open) != Some(&'[') {
                    out.push(c);
                    i += 1;
                    continue;
                }
                match (find_close_bracket(&chars, open), find_url(&chars, open)) {
                    (Some(_), Some((text_end, url_end))) => {
                        if !is_image {
                            out.extend(&chars[open + 1..text_end]);
                        }
                        i = url_end;
                    }
                    _ => {
                        out.push(c);
                        i += 1;
                    }
                }
            }
            '*' | '_' | '~' => {
                let len = run_len(i, c);
                // A run that closes the most recently opened run of the same
                // marker is a closer; anything else is only an opener when a
                // matching run exists later in the line.
                if open_delims.last() == Some(&(c, len)) {
                    open_delims.pop();
                    i += len;
                    continue;
                }
                if find_run(i + len, c, len).is_some() {
                    open_delims.push((c, len));
                    i += len;
                } else {
                    out.push(c);
                    i += 1;
                }
            }
            '<' => {
                match chars[i..].iter().position(|&x| x == '>') {
                    Some(offset) => {
                        let inner: String = chars[i + 1..i + offset].iter().collect();
                        // An autolink carries a URL; a tag carries nothing.
                        if inner.starts_with("http") || inner.contains('@') {
                            out.push_str(inner.trim_start_matches('/').trim_start_matches(':'));
                        }
                        i += offset + 1;
                    }
                    None => {
                        out.push(c);
                        i += 1;
                    }
                }
            }
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
    out
}

fn find_close_bracket(chars: &[char], open: usize) -> Option<usize> {
    chars[open..]
        .iter()
        .position(|&c| c == ']')
        .map(|p| open + p)
}

/// Locate `(url)` after a `]`, returning the end of the label and of the url.
fn find_url(chars: &[char], open: usize) -> Option<(usize, usize)> {
    let close = find_close_bracket(chars, open)?;
    let start = *chars.get(close + 1)?;
    if start != '(' {
        return None;
    }
    let mut depth = 0usize;
    for (offset, &c) in chars[close + 1..].iter().enumerate() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some((close, close + 1 + offset + 1));
                }
            }
            _ => {}
        }
    }
    None
}

/// Flush the paragraph being accumulated as a block, if there is one.
fn flush_paragraph(para: &mut Vec<String>, stack: &SectionStack, drafts: &mut Vec<Draft>) {
    if para.is_empty() {
        return;
    }
    let content = join_lines(para);
    para.clear();
    if content.trim().is_empty() {
        return;
    }
    drafts.push(Draft {
        content,
        block_type: "paragraph",
        section_path: stack.path(),
        title: None,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn md(text: &str) -> DocumentData {
        parse_markdown("测试", text).unwrap()
    }

    #[test]
    fn test_headings_become_title_blocks_with_a_section_path() {
        let doc = md("# 第一章 出发\n\n他走了。\n\n## 小节\n\n又走了。\n");
        let types: Vec<&str> = doc.blocks.iter().map(|b| b.block_type.as_str()).collect();
        assert_eq!(types, ["title", "paragraph", "title", "paragraph"]);
        assert_eq!(doc.blocks[0].title.as_deref(), Some("第一章 出发"));
        assert_eq!(doc.blocks[1].section_path, "第一章 出发");
        assert_eq!(doc.blocks[2].section_path, "第一章 出发 / 小节");
    }

    #[test]
    fn test_a_deeper_heading_then_a_shallower_one_pops_the_stack() {
        let doc = md("# 甲\n\n## 乙\n\n### 丙\n\n# 丁\n\n正文。\n");
        assert_eq!(doc.blocks.last().unwrap().section_path, "丁");
    }

    #[test]
    fn test_inline_markup_is_stripped_and_the_text_kept() {
        let doc = md("这是**重点**，还有[链接文字](http://example.com/a)与`代码`。\n");
        assert_eq!(doc.blocks[0].content, "这是重点，还有链接文字与代码。");
    }

    #[test]
    fn test_an_image_keeps_nothing_and_a_link_keeps_its_text() {
        let doc = md("看图 ![图注](img/a.png) 与 [出处](b.md)。\n");
        assert_eq!(doc.blocks[0].content, "看图  与 出处。");
    }

    #[test]
    fn test_an_unpaired_marker_is_left_alone() {
        // A stray asterisk is the author's, not emphasis.
        let doc = md("他说了*这句话。\n");
        assert_eq!(doc.blocks[0].content, "他说了*这句话。");
    }

    #[test]
    fn test_front_matter_is_dropped() {
        let doc = md("---\ntitle: 书\n---\n\n正文。\n");
        assert_eq!(doc.blocks.len(), 1);
        assert_eq!(doc.blocks[0].content, "正文。");
    }

    #[test]
    fn test_a_thematic_break_is_not_a_paragraph() {
        let doc = md("上半场。\n\n---\n\n下半场。\n");
        assert_eq!(doc.blocks.len(), 2);
        assert!(doc.blocks.iter().all(|b| !b.content.contains('-')));
    }

    #[test]
    fn test_a_scene_break_written_in_dashes_is_not_dialogue() {
        // The dialogue pass reads a leading `——` as a spoken line; a rule of
        // dashes must never reach it.
        let doc = md("第一段。\n\n———\n\n第二段。\n");
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_list_items_are_their_own_blocks() {
        let doc = md("- 第一\n- 第二\n1. 第三\n");
        let types: Vec<&str> = doc.blocks.iter().map(|b| b.block_type.as_str()).collect();
        assert_eq!(types, ["list", "list", "list"]);
        assert_eq!(doc.blocks[0].content, "第一");
    }

    #[test]
    fn test_a_negative_number_is_not_a_list_item() {
        let doc = md("温度是 -3.5 度。\n");
        assert_eq!(doc.blocks[0].block_type, "paragraph");
        assert_eq!(doc.blocks[0].content, "温度是 -3.5 度。");
    }

    #[test]
    fn test_wrapped_lines_are_one_paragraph_not_two_sentences() {
        // Joined without a space for CJK: `split_sentences` ends a sentence at a
        // newline, so a wrap must not survive as one.
        let doc = md("出版学是研究出版\n活动的学科。\n");
        assert_eq!(doc.blocks[0].content, "出版学是研究出版活动的学科。");
        assert!(!doc.blocks[0].content.contains('\n'));
    }

    #[test]
    fn test_wrapped_english_lines_keep_their_space() {
        let doc = md("The quick brown\nfox jumps.\n");
        assert_eq!(doc.blocks[0].content, "The quick brown fox jumps.");
    }

    #[test]
    fn test_chapter_line_without_markup_is_a_heading() {
        let doc = md("第三十一章 归途\n\n他回来了。\n");
        assert_eq!(doc.blocks[0].block_type, "title");
        assert_eq!(doc.blocks[1].section_path, "第三十一章 归途");
    }

    #[test]
    fn test_a_prose_line_that_starts_like_a_chapter_is_not_a_heading() {
        for line in ["第一天，他推开门。", "第三章的某个夜晚", "第二"] {
            assert_eq!(chapter_heading(line), None, "{line}");
        }
    }

    #[test]
    fn test_fenced_content_is_kept_verbatim() {
        let doc = md("```\n**字面**\n```\n");
        assert_eq!(doc.blocks[0].content, "**字面**");
    }

    #[test]
    fn test_no_nlp_payload_is_invented() {
        let doc = md("# 章\n\n正文一段。\n");
        assert!(doc.blocks.iter().all(|b| b.tokens.is_empty()));
        assert!(doc.blocks.iter().all(|b| b.entities.is_empty()));
        assert!(doc.blocks.iter().all(|b| b.noun_signals.is_empty()));
        assert_eq!(doc.total_word_count, 0);
    }

    #[test]
    fn test_char_count_is_bytes_like_the_semantic_importer() {
        let doc = md("出版\n");
        // "出版" is 6 bytes, plus the newline the block boundary removed: the
        // sum is over saved block contents, so this is 6.
        assert_eq!(doc.total_char_count, 6);
    }

    #[test]
    fn test_crlf_is_normalized() {
        let doc = md("第一行。\r\n第二行。\r\n");
        assert!(!doc.blocks[0].content.contains('\r'));
    }

    #[test]
    fn test_html_comments_are_removed() {
        let doc = md("前<!-- 编辑注 -->后\n");
        assert_eq!(doc.blocks[0].content, "前后");
    }

    #[test]
    fn test_txt_reads_one_paragraph_per_line_without_blank_lines() {
        let doc = parse_txt("书", "第一章\n他走了。\n她又走了。\n").unwrap();
        let contents: Vec<&str> = doc.blocks.iter().map(|b| b.content.as_str()).collect();
        assert_eq!(contents, ["第一章", "他走了。", "她又走了。"]);
        assert_eq!(doc.blocks[1].section_path, "第一章");
    }

    #[test]
    fn test_txt_with_blank_lines_groups_paragraphs() {
        let doc = parse_txt("书", "第一段前半\n第一段后半。\n\n第二段。\n").unwrap();
        let contents: Vec<&str> = doc.blocks.iter().map(|b| b.content.as_str()).collect();
        assert_eq!(contents, ["第一段前半第一段后半。", "第二段。"]);
    }

    #[test]
    fn test_txt_does_not_strip_markup() {
        // In a .txt file an asterisk is a footnote mark or a scene break, not
        // emphasis; deleting it would be a guess about the author's intent.
        let doc = parse_txt("书", "正文*注。\n").unwrap();
        assert_eq!(doc.blocks[0].content, "正文*注。");
    }

    #[test]
    fn test_empty_input_yields_no_blocks() {
        let doc = md("\n\n   \n");
        assert!(doc.blocks.is_empty());
        assert_eq!(doc.total_char_count, 0);
    }

    #[test]
    fn test_block_ids_are_left_for_the_database() {
        // A fresh id per saved block is what keeps two imports of the same file
        // from colliding on the primary key.
        let doc = md("一段。\n\n两段。\n");
        assert!(doc.blocks.iter().all(|b| b.source_block_id.is_empty()));
    }
}
