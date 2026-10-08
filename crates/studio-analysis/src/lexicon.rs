//! Shared lexicon matcher: a two-line pattern language and a longest-match scan.
//!
//! Two dimensions ask the same question — where does this text say one of *these*
//! things? — so they share one matcher: 陈词滥调 looks for stock phrasing, 显式语言 for
//! content a reader might want warned about. The matcher knows patterns only. What a
//! pattern means (its category and advice) stays with the dimension that owns the
//! library, reachable at the same index the matcher reports.
//!
//! The pattern language is deliberately tiny: literal characters plus one level of
//! alternation groups — `(淡淡|浅浅)一笑` — where a group may be skipped by writing `?`
//! after it. A full regex would be more expressive and much easier to write something
//! with that silently matches half the book.
//!
//! Deterministic: variants are expanded longest-first, the scan never produces
//! overlapping matches, and the first-character index keeps a pass over a novel
//! linear instead of trying every pattern at every position.

use std::collections::HashMap;

/// How many character sequences one pattern may expand to. A guard against a
/// careless pattern, not a limit anyone is meant to reach.
pub const MAX_VARIANTS: usize = 16;
/// Characters of context kept on either side of a hit, so it can be judged in place.
pub const EXCERPT_RADIUS: usize = 24;

/// One library entry after its alternation groups have been expanded.
#[derive(Debug, Clone)]
pub struct Compiled {
    /// The pattern as written in the library.
    pub pattern: &'static str,
    /// What family the entry belongs to, as the owning dimension defines it.
    pub category: &'static str,
    /// How strong the entry is, as the owning dimension defines it. Empty for a
    /// library that names its entries rather than grading them, as 陈词滥调's does.
    pub tier: &'static str,
    /// What the dimension advises about it.
    pub note: &'static str,
    /// The character sequences the pattern stands for, longest first.
    pub variants: Vec<Vec<char>>,
}

/// One hit before the owning dimension places it in its output.
#[derive(Debug, Clone)]
pub struct Match {
    /// Position of the matched entry in the library the scan used.
    pub pattern_idx: usize,
    /// The characters actually found.
    pub matched: String,
    /// Character offset inside the block.
    pub char_offset: usize,
    /// Surrounding characters, with ellipses where they were cut.
    pub excerpt: String,
    /// The hit sits inside quotation marks: someone says it rather than narrates it.
    pub in_dialogue: bool,
}

/// Patterns sharing a first character, longest variant first: `(entry, variant)`.
pub type Index = HashMap<char, Vec<(usize, usize)>>;

/// Expand one pattern's alternation groups into the variants it stands for.
///
/// `(淡淡|浅浅)一笑` yields `淡淡一笑` and `浅浅一笑`; a `?` after a group means the
/// group may also be absent.
pub fn expand(pattern: &str) -> Vec<Vec<char>> {
    let mut current: Vec<Vec<char>> = vec![Vec::new()];
    let mut chars = pattern.chars().peekable();

    while let Some(c) = chars.next() {
        if c != '(' {
            for variant in &mut current {
                variant.push(c);
            }
            continue;
        }

        let mut body = String::new();
        while let Some(&c) = chars.peek() {
            if c == ')' {
                chars.next();
                break;
            }
            body.push(c);
            chars.next();
        }

        let mut options: Vec<Vec<char>> = body
            .split('|')
            .map(|part| part.chars().collect::<Vec<char>>())
            .collect();
        if chars.peek() == Some(&'?') {
            chars.next();
            options.push(Vec::new());
        }

        let mut next: Vec<Vec<char>> = Vec::new();
        for prefix in &current {
            for option in &options {
                let mut merged = prefix.clone();
                merged.extend(option.iter().copied());
                next.push(merged);
            }
        }
        next.truncate(MAX_VARIANTS);
        current = next;
    }

    current.retain(|v| !v.is_empty());
    // Longest first: the scanner relies on this to take the longest match at a
    // position, which is what stops overlapping patterns from double counting.
    current.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));
    current
}

/// Compile a library for scanning. Library order is preserved, so an index the
/// matcher reports can be read straight back against the library.
/// Compile a library that does not grade its entries: the tier slot stays empty.
pub fn compile(library: &[(&'static str, &'static str, &'static str)]) -> Vec<Compiled> {
    library
        .iter()
        .map(|(pattern, category, note)| Compiled {
            pattern,
            category,
            tier: "",
            note,
            variants: expand(pattern),
        })
        .collect()
}

/// Compile a library whose entries carry a strength tier, as 显式语言's does:
/// `pattern, category, tier, note`.
pub fn compile_graded(
    library: &[(&'static str, &'static str, &'static str, &'static str)],
) -> Vec<Compiled> {
    library
        .iter()
        .map(|(pattern, category, tier, note)| Compiled {
            pattern,
            category,
            tier,
            note,
            variants: expand(pattern),
        })
        .collect()
}

/// Map first character → `(entry index, variant index)`, longest variant first.
///
/// Indexing on the first character keeps a full pass over a novel linear: at each
/// position only the handful of patterns starting with that character are tried.
pub fn build_index(lexicon: &[Compiled]) -> Index {
    let mut index: Index = HashMap::new();
    for (p, entry) in lexicon.iter().enumerate() {
        for (v, variant) in entry.variants.iter().enumerate() {
            if let Some(&first) = variant.first() {
                index.entry(first).or_default().push((p, v));
            }
        }
    }
    for candidates in index.values_mut() {
        candidates.sort_by(|a, b| {
            let la = lexicon[a.0].variants[a.1].len();
            let lb = lexicon[b.0].variants[b.1].len();
            lb.cmp(&la).then_with(|| a.cmp(b))
        });
    }
    index
}

/// Whether `variant` fits at `at`.
fn fits(chars: &[char], at: usize, variant: &[char]) -> bool {
    at + variant.len() <= chars.len() && chars[at..at + variant.len()] == *variant
}

/// Opening and closing quotation marks, as the dialogue measures elsewhere treat them.
fn is_open_quote(c: char) -> bool {
    matches!(c, '“' | '「' | '『' | '"')
}

fn is_close_quote(c: char) -> bool {
    matches!(c, '”' | '」' | '』')
}

fn apply_quote(c: char, in_quote: &mut bool) {
    if is_open_quote(c) {
        *in_quote = true;
    } else if is_close_quote(c) {
        *in_quote = false;
    }
}

/// A window of context around a hit, with ellipses where characters were cut.
pub fn excerpt(chars: &[char], at: usize, len: usize) -> String {
    let start = at.saturating_sub(EXCERPT_RADIUS);
    let end = (at + len + EXCERPT_RADIUS).min(chars.len());
    let mut out = String::new();
    if start > 0 {
        out.push('…');
    }
    out.extend(chars[start..end].iter());
    if end < chars.len() {
        out.push('…');
    }
    out
}

/// Walk one block and collect its hits.
///
/// The walk advances one character at a time except past a match, so hits never
/// overlap and a nested pattern cannot be counted twice. Quotation state is tracked
/// in the same pass: a hit inside 「」 is someone's line rather than the narration's,
/// and that difference is reported instead of resolved.
pub fn scan_block(content: &str, index: &Index, lexicon: &[Compiled]) -> Vec<Match> {
    let chars: Vec<char> = content.chars().collect();
    let mut hits = Vec::new();
    let mut in_quote = false;
    let mut i = 0usize;

    while i < chars.len() {
        apply_quote(chars[i], &mut in_quote);
        let mut step = 1usize;

        if let Some(candidates) = index.get(&chars[i]) {
            for &(p, v) in candidates {
                let variant = &lexicon[p].variants[v];
                if fits(&chars, i, variant) {
                    hits.push(Match {
                        pattern_idx: p,
                        matched: variant.iter().collect(),
                        char_offset: i,
                        excerpt: excerpt(&chars, i, variant.len()),
                        in_dialogue: in_quote,
                    });
                    step = variant.len();
                    break;
                }
            }
        }

        // Quotation state has to stay correct across the characters the match skipped.
        for c in &chars[i + 1..i + step] {
            apply_quote(*c, &mut in_quote);
        }
        i += step;
    }

    hits
}
