//! Print-ready HTML for a stored Markdown report — the file the user prints
//! to PDF.
//!
//! This is a rendering of the *stored Markdown*, not a second renderer over the
//! analysis. That choice is deliberate:
//!
//! - `render_markdown` owns the wording of a report. A parallel HTML renderer
//!   would be a second place to phrase the same verdict, and the two would
//!   drift — exactly the failure this codebase refuses elsewhere (the UI mirrors
//!   the report's labels for the same reason).
//! - The HTML is a pure function of text already in the database, so a second
//!   column would only add a copy that can rot.
//! - The numbers therefore still come from the report as generated: printing
//!   later cannot quietly pick up a recomputed analysis.
//!
//! The converter covers exactly the Markdown `markdown.rs` emits: `#`–`###`
//! headings, `- ` bullet lists, pipe tables with `---:` alignment markers,
//! `---` rules, `_em_`, `**strong**`, `` `code` `` and plain lines. It is not a
//! general Markdown implementation and refuses to guess at anything else: an
//! unknown line becomes escaped text, never markup. Everything is HTML-escaped,
//! so nothing an analysed document contains can reach the file as a tag.
//!
//! Output is deterministic — same Markdown, same bytes — which is what lets the
//! tests below pin it.

/// Render the report Markdown as a self-contained, print-ready HTML document.
pub fn render_html(markdown: &str) -> String {
    let blocks = parse_blocks(markdown);

    // The document title is the report's own H1; a report without one still has
    // to produce a browser tab with a name.
    let title = blocks.iter().find_map(|block| match block {
        Block::Heading(1, text) => Some(inline_html(text)),
        _ => None,
    });

    let mut out = String::new();
    out.push_str("<!DOCTYPE html>\n<html lang=\"zh-CN\">\n<head>\n<meta charset=\"utf-8\" />\n");
    match title {
        Some(title) => out.push_str(&format!("<title>{title}</title>\n")),
        None => out.push_str("<title>NarrativeStudio 分析报告</title>\n"),
    }
    out.push_str(STYLE);
    out.push_str("</head>\n<body>\n");
    out.push_str(HINT);

    for block in &blocks {
        match block {
            Block::Heading(level, text) => {
                out.push_str(&format!("<h{level}>{}</h{level}>\n", inline_html(text)));
            }
            Block::Paragraph(text) => {
                // Consecutive plain lines stay separate visual lines: a report
                // writes one sentence per line, and joining them with a space
                // would insert one in the middle of Chinese text.
                let lines: Vec<String> = text.lines().map(inline_html).collect();
                out.push_str(&format!("<p>{}</p>\n", lines.join("<br />\n")));
            }
            Block::List(items) => {
                out.push_str("<ul>\n");
                for item in items {
                    out.push_str(&format!("<li>{}</li>\n", inline_html(item)));
                }
                out.push_str("</ul>\n");
            }
            Block::Table {
                header,
                align,
                rows,
            } => {
                out.push_str("<table>\n<thead>\n<tr>");
                for (cell, alignment) in header.iter().zip(align) {
                    out.push_str(&format!(
                        "<th{}>{}</th>",
                        alignment.attr(),
                        inline_html(cell)
                    ));
                }
                out.push_str("</tr>\n</thead>\n<tbody>\n");
                for row in rows {
                    out.push_str("<tr>");
                    for (index, alignment) in align.iter().enumerate().take(header.len()) {
                        // A short row is padded rather than shifted: a missing
                        // cell must read as empty, not move the columns left.
                        let cell = row.get(index).copied().unwrap_or("");
                        out.push_str(&format!(
                            "<td{}>{}</td>",
                            alignment.attr(),
                            inline_html(cell)
                        ));
                    }
                    out.push_str("</tr>\n");
                }
                out.push_str("</tbody>\n</table>\n");
            }
            Block::Divider => out.push_str("<hr />\n"),
        }
    }

    out.push_str("</body>\n</html>\n");
    out
}

/// The block kinds the report Markdown uses.
enum Block<'a> {
    Heading(usize, &'a str),
    Paragraph(String),
    List(Vec<&'a str>),
    Table {
        header: Vec<&'a str>,
        align: Vec<Align>,
        rows: Vec<Vec<&'a str>>,
    },
    Divider,
}

#[derive(Clone, Copy)]
enum Align {
    Left,
    Center,
    Right,
}

impl Align {
    fn attr(self) -> &'static str {
        match self {
            Align::Left | Align::Center => "",
            Align::Right => " class=\"right\"",
        }
    }
}

fn parse_blocks(markdown: &str) -> Vec<Block<'_>> {
    let lines: Vec<&str> = markdown.lines().collect();
    let mut blocks = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        let line = lines[index].trim_end();
        if line.trim().is_empty() {
            index += 1;
            continue;
        }

        if let Some((level, text)) = heading(line) {
            blocks.push(Block::Heading(level, text));
            index += 1;
            continue;
        }
        if line.trim() == "---" {
            blocks.push(Block::Divider);
            index += 1;
            continue;
        }
        if let Some(item) = line.strip_prefix("- ") {
            let mut items = vec![item.trim()];
            index += 1;
            while let Some(next) = lines.get(index).map(|line| line.trim_end()) {
                match next.strip_prefix("- ") {
                    Some(item) => items.push(item.trim()),
                    None => break,
                }
                index += 1;
            }
            blocks.push(Block::List(items));
            continue;
        }
        let followed_by_separator = lines
            .get(index + 1)
            .is_some_and(|next| is_table_separator(next));
        if line.starts_with('|') && followed_by_separator {
            let header = split_row(line);
            let align = aligns(lines[index + 1]);
            index += 2;
            let mut rows = Vec::new();
            while lines
                .get(index)
                .is_some_and(|line| line.trim_end().starts_with('|'))
            {
                rows.push(split_row(lines[index].trim_end()));
                index += 1;
            }
            blocks.push(Block::Table {
                header,
                align,
                rows,
            });
            continue;
        }

        // Plain text: run to the blank line that ends the paragraph.
        let start = index;
        while index < lines.len() {
            let next = lines[index].trim_end();
            if next.trim().is_empty()
                || heading(next).is_some()
                || next.trim() == "---"
                || next.starts_with("- ")
                || next.starts_with('|')
            {
                break;
            }
            index += 1;
        }
        if index == start {
            // A line nothing else claimed (a stray `|` row with no separator,
            // say) still has to be emitted, and it is emitted as escaped text.
            blocks.push(Block::Paragraph(line.trim().to_string()));
            index += 1;
        } else {
            let text = lines[start..index]
                .iter()
                .map(|line| line.trim())
                .collect::<Vec<_>>()
                .join("\n");
            blocks.push(Block::Paragraph(text));
        }
    }

    blocks
}

fn heading(line: &str) -> Option<(usize, &str)> {
    let hashes = line.chars().take_while(|c| *c == '#').count();
    if hashes == 0 || hashes > 3 {
        return None;
    }
    let text = line[hashes..].strip_prefix(' ')?;
    Some((hashes, text.trim()))
}

/// `| --- | ---: |` and nothing else.
fn is_table_separator(line: &str) -> bool {
    let line = line.trim();
    if !line.starts_with('|') {
        return false;
    }
    let cells = split_row(line);
    !cells.is_empty()
        && cells.iter().all(|cell| {
            let body = cell.trim();
            let body = body.strip_prefix(':').unwrap_or(body);
            let body = body.strip_suffix(':').unwrap_or(body);
            let body = body.trim();
            !body.is_empty() && body.chars().all(|c| c == '-')
        })
}

fn aligns(line: &str) -> Vec<Align> {
    split_row(line)
        .iter()
        .map(|cell| {
            let cell = cell.trim();
            match (cell.starts_with(':'), cell.ends_with(':')) {
                (true, true) => Align::Center,
                (false, true) => Align::Right,
                _ => Align::Left,
            }
        })
        .collect()
}

/// Split a table row on unescaped `|`. `markdown.rs` escapes a literal pipe in
/// analysed text as `\|` so it cannot end a cell; here it becomes a plain pipe
/// again, which HTML needs no escaping for.
fn split_row(line: &str) -> Vec<&str> {
    let body = line.trim().trim_matches('|');
    let mut cells = Vec::new();
    let mut start = 0;
    let mut index = 0;
    let bytes = body.as_bytes();
    while index < bytes.len() {
        match bytes[index] {
            b'\\' if index + 1 < bytes.len() && bytes[index + 1] == b'|' => index += 2,
            b'|' => {
                cells.push(body[start..index].trim());
                index += 1;
                start = index;
            }
            _ => index += 1,
        }
    }
    cells.push(body[start..].trim());
    cells
}

/// Inline Markdown: escape everything, then honour `` `code` ``, `**strong**`
/// and `_em_`. Emphasis is not nested, because the report never nests it.
fn inline_html(raw: &str) -> String {
    let chars: Vec<char> = raw.chars().collect();
    let mut out = String::new();
    let mut index = 0;

    while index < chars.len() {
        match chars[index] {
            '&' => {
                out.push_str("&amp;");
                index += 1;
            }
            '<' => {
                out.push_str("&lt;");
                index += 1;
            }
            '>' => {
                out.push_str("&gt;");
                index += 1;
            }
            '\\' if chars.get(index + 1) == Some(&'|') => {
                out.push('|');
                index += 2;
            }
            '`' => match find_char(&chars, index + 1, '`') {
                Some(end) => {
                    out.push_str("<code>");
                    out.push_str(&escape(&chars[index + 1..end].iter().collect::<String>()));
                    out.push_str("</code>");
                    index = end + 1;
                }
                None => {
                    out.push('`');
                    index += 1;
                }
            },
            '*' if chars.get(index + 1) == Some(&'*') => match find_pair(&chars, index + 2, '*') {
                Some(end) => {
                    out.push_str("<strong>");
                    out.push_str(&escape(&chars[index + 2..end].iter().collect::<String>()));
                    out.push_str("</strong>");
                    index = end + 2;
                }
                None => {
                    out.push('*');
                    index += 1;
                }
            },
            '_' if is_emphasis_open(&chars, index) => {
                match find_emphasis_close(&chars, index + 1) {
                    Some(end) => {
                        out.push_str("<em>");
                        out.push_str(&escape(&chars[index + 1..end].iter().collect::<String>()));
                        out.push_str("</em>");
                        index = end + 1;
                    }
                    None => {
                        out.push('_');
                        index += 1;
                    }
                }
            }
            other => {
                out.push(other);
                index += 1;
            }
        }
    }

    out
}

/// `_` opens emphasis only between words — so a key written without backticks
/// (`timeline_analysis`) cannot turn half a report into italics.
fn is_emphasis_open(chars: &[char], index: usize) -> bool {
    let before_ok = index == 0 || !is_word(chars[index - 1]);
    let next_ok = chars
        .get(index + 1)
        .is_some_and(|c| !c.is_whitespace() && *c != '_');
    before_ok && next_ok
}

fn find_emphasis_close(chars: &[char], from: usize) -> Option<usize> {
    (from..chars.len())
        .find(|&i| chars[i] == '_' && (i + 1 >= chars.len() || !is_word(chars[i + 1])))
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '`'
}

fn find_char(chars: &[char], from: usize, target: char) -> Option<usize> {
    (from..chars.len()).find(|&i| chars[i] == target)
}

fn find_pair(chars: &[char], from: usize, target: char) -> Option<usize> {
    (from..chars.len().saturating_sub(1)).find(|&i| chars[i] == target && chars[i + 1] == target)
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// A4 with generous margins, a CJK-capable font stack, table headers that repeat
/// across pages, and no row split across a page break. Colours are set to print,
/// not only to look right on screen.
const STYLE: &str = r#"<style>
@page { size: A4; margin: 16mm 14mm 18mm; }
html { -webkit-print-color-adjust: exact; print-color-adjust: exact; }
body { margin: 0 auto; max-width: 182mm; padding: 24px; color: #1f2328;
  font: 10.5pt/1.75 -apple-system, "Helvetica Neue", "PingFang SC", "Hiragino Sans GB",
        "Microsoft YaHei", "Noto Sans CJK SC", sans-serif; }
h1 { font-size: 19pt; margin: 0 0 1.1em; }
h2 { font-size: 14pt; margin: 1.9em 0 .6em; padding-bottom: .25em;
  border-bottom: 1px solid #d8dce2; break-after: avoid; }
h3 { font-size: 11.5pt; margin: 1.4em 0 .4em; break-after: avoid; }
p, li { orphans: 2; widows: 2; }
ul { margin: .4em 0 1em; padding-left: 1.5em; }
li { margin: .15em 0; }
code { font-family: ui-monospace, Menlo, Consolas, monospace; font-size: .88em;
  background: #f2f4f7; padding: 0 .25em; border-radius: 3px; }
hr { border: 0; border-top: 1px solid #d8dce2; margin: 2em 0; }
table { width: 100%; border-collapse: collapse; margin: .5em 0 1.2em; font-size: 9pt; }
thead { display: table-header-group; }
tr { break-inside: avoid; }
th, td { border: .5pt solid #c9ccd1; padding: .3em .5em; text-align: left;
  vertical-align: top; }
th { background: #f2f4f7; font-weight: 600; }
.right { text-align: right; }
.print-hint { background: #eef4ff; border: 1px solid #c4d6f5; border-radius: 6px;
  padding: .6em .8em; font-size: 9.5pt; }
@media print { .print-hint { display: none; } body { max-width: none; padding: 0; } }
</style>
"#;

/// Shown on screen, hidden in print: what to press to get the PDF.
const HINT: &str = "<p class=\"print-hint\">用浏览器打开此文件后按 ⌘P（Windows 为 Ctrl+P），\
选择「另存为 PDF」即可。纸张已设为 A4，表格跨页会重复表头。</p>\n";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn golden_minimal_document() {
        let html = render_html("# 书 · 叙事分析报告\n\n总体评分：**82.0 / 100**\n");
        assert!(html.starts_with("<!DOCTYPE html>\n<html lang=\"zh-CN\">\n"));
        assert!(html.contains("<title>书 · 叙事分析报告</title>"));
        assert!(html.contains("<h1>书 · 叙事分析报告</h1>"));
        assert!(html.contains("<p>总体评分：<strong>82.0 / 100</strong></p>"));
        assert!(html.ends_with("</body>\n</html>\n"));
    }

    #[test]
    fn the_same_markdown_gives_the_same_bytes() {
        let md = "# 报告\n\n## 总体评估\n\n| 维度 | 得分 |\n| --- | ---: |\n| 节奏 | 70.0 |\n";
        assert_eq!(render_html(md), render_html(md));
    }

    #[test]
    fn nothing_from_the_document_becomes_markup() {
        let html =
            render_html("| 原文 | 备注 |\n| --- | --- |\n| <script>alert(1)</script> | a & b |\n");
        assert!(!html.contains("<script"));
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(html.contains("a &amp; b"));
    }

    #[test]
    fn an_escaped_pipe_stays_inside_its_cell() {
        let html = render_html("| 名称 | 次数 |\n| --- | ---: |\n| a\\|b | 3 |\n");
        // The cell count is what matters: a stray pipe would have split the row.
        assert_eq!(html.matches("<td").count(), 2, "{html}");
        assert!(html.contains("<td>a|b</td>"));
    }

    #[test]
    fn alignment_markers_become_a_class_not_a_column() {
        let html = render_html("| a | b |\n| --- | ---: |\n| 1 | 2 |\n");
        assert!(html.contains("<th class=\"right\">b</th>"));
        assert!(html.contains("<td class=\"right\">2</td>"));
        assert!(!html.contains("---"), "the marker row must not be printed");
    }

    #[test]
    fn a_short_row_leaves_the_columns_where_they_are() {
        let html = render_html("| a | b | c |\n| --- | --- | --- |\n| 1 | 2 |\n");
        assert_eq!(html.matches("<td").count(), 3);
    }

    #[test]
    fn lists_rules_and_notes_survive() {
        let html = render_html(
            "- 生成时间：2026-01-01 08:00 UTC\n- 分析版本：14\n\n---\n\n_（另有 3 项未列出）_\n",
        );
        assert!(html.contains("<li>生成时间：2026-01-01 08:00 UTC</li>"));
        assert!(html.contains("<hr />"));
        assert!(html.contains("<em>（另有 3 项未列出）</em>"));
    }

    #[test]
    fn a_key_written_without_backticks_is_not_italics() {
        let html = render_html("见 timeline_analysis 一节。\n");
        assert!(!html.contains("<em>"), "{html}");
        assert!(html.contains("timeline_analysis"));
    }

    #[test]
    fn a_key_in_backticks_stays_a_key() {
        let html = render_html("见 `timeline_analysis` 一节。\n");
        assert!(html.contains("<code>timeline_analysis</code>"));
        assert!(!html.contains("<em>"), "{html}");
    }

    #[test]
    fn every_table_tag_is_closed() {
        let html = render_html(
            "# 报告\n\n## 情节\n\n| 线 | 形 |\n| --- | --- |\n| A | 贯穿 |\n\n### 明细\n\n- x\n",
        );
        assert_eq!(
            html.matches("<table>").count(),
            html.matches("</table>").count()
        );
        assert_eq!(html.matches("<tr>").count(), html.matches("</tr>").count());
        assert_eq!(html.matches("<ul>").count(), html.matches("</ul>").count());
        assert_eq!(
            html.matches("<thead>").count(),
            html.matches("</thead>").count()
        );
    }

    #[test]
    fn the_print_setup_and_hint_are_present() {
        let html = render_html("# 报告\n");
        assert!(html.contains("@page { size: A4"));
        assert!(html.contains("display: table-header-group"));
        assert!(html.contains("class=\"print-hint\""));
        assert!(html.contains("@media print { .print-hint { display: none; }"));
    }
}
