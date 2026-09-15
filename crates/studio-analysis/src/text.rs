//! Shared text utilities for the analysis engine.

/// Split text into sentences on CJK/Latin terminal punctuation and newlines.
///
/// The terminal punctuation itself is not included in the returned sentences.
/// Empty fragments (whitespace-only) are dropped.
pub fn split_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current = String::new();
    for c in text.chars() {
        match c {
            '。' | '！' | '？' | '；' | '…' | '!' | '?' | ';' | '\n' => {
                if !current.trim().is_empty() {
                    sentences.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(c),
        }
    }
    if !current.trim().is_empty() {
        sentences.push(current);
    }
    sentences
}

/// Whether a character is a CJK ideograph (main ranges + Extension A).
pub fn is_cjk(c: char) -> bool {
    matches!(
        c as u32,
        0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0xF900..=0xFAFF
    )
}

/// Count non-whitespace characters that appear inside CJK/Latin quotation
/// marks (a rough dialogue detector: “ ”, 「 」, 『 』, " ").
pub fn count_quoted_chars(text: &str) -> usize {
    let mut count = 0;
    let mut in_quote = false;
    for c in text.chars() {
        match c {
            '“' | '「' | '『' | '"' => in_quote = true,
            '”' | '」' | '』' => in_quote = false,
            _ => {
                if in_quote && !c.is_whitespace() {
                    count += 1;
                }
            }
        }
    }
    count
}

/// Count non-whitespace characters in a string.
pub fn visible_char_count(text: &str) -> usize {
    text.chars().filter(|c| !c.is_whitespace()).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_sentences_cjk() {
        let s = split_sentences("出版学是研究出版活动的学科。它涵盖编辑、印刷与发行。");
        assert_eq!(s.len(), 2);
        assert_eq!(s[0], "出版学是研究出版活动的学科");
    }

    #[test]
    fn test_split_sentences_mixed() {
        let s = split_sentences("First line!\nSecond? Third; fourth.");
        assert_eq!(s.len(), 4);
    }

    #[test]
    fn test_split_sentences_empty() {
        assert!(split_sentences("   \n  ").is_empty());
    }

    #[test]
    fn test_is_cjk() {
        assert!(is_cjk('出'));
        assert!(is_cjk('版'));
        assert!(!is_cjk('a'));
        assert!(!is_cjk('。'));
    }

    #[test]
    fn test_count_quoted_chars() {
        // 你好。 inside the quotes (punctuation counts as visible).
        assert_eq!(count_quoted_chars("他说：“你好。”然后走了。"), 3);
        assert_eq!(count_quoted_chars("他说：“你好 吗。”"), 4);
        assert_eq!(count_quoted_chars("no quotes here"), 0);
    }

    #[test]
    fn test_visible_char_count() {
        assert_eq!(visible_char_count("ab cd\ne"), 5);
    }
}
