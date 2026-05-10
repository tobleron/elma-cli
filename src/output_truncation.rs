//! @efficiency-role: domain-logic
//! Unified Truncation Policy (Task 787)
//!
//! Provides sophisticated truncation strategies beyond simple character slicing.
//! Used by tools to produce clean, model-friendly previews.

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(crate) enum TruncationPolicy {
    /// Show first N characters.
    Head(usize),
    /// Show last N characters.
    Tail(usize),
    /// Show first N and last M characters with a placeholder in between.
    HeadAndTail(usize, usize),
    /// Show first N lines.
    Lines(usize),
    /// Show first N characters, truncated at last complete sentence.
    Sentence(usize),
}

impl Default for TruncationPolicy {
    fn default() -> Self {
        TruncationPolicy::Head(50_000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_text_head() {
        let text = "Hello, world!";
        assert_eq!(truncate_text(text, TruncationPolicy::Head(5)), "Hello... [truncated, 13 total chars]");
        assert_eq!(truncate_text(text, TruncationPolicy::Head(20)), "Hello, world!");
    }

    #[test]
    fn test_truncate_text_sentence() {
        let text = "First sentence. Second sentence! Third sentence? Fourth sentence\nFifth sentence.";

        // Truncate at 20 chars, should find the first period
        let result = truncate_text(text, TruncationPolicy::Sentence(20));
        assert_eq!(result, "First sentence. (truncated)");

        // Truncate at 40 chars, should find the exclamation mark
        let result = truncate_text(text, TruncationPolicy::Sentence(40));
        assert_eq!(result, "First sentence. Second sentence! (truncated)");

        // Truncate at 70 chars, should find the newline
        let result = truncate_text(text, TruncationPolicy::Sentence(70));
        assert_eq!(result, "First sentence. Second sentence! Third sentence? Fourth sentence (truncated)");

        // Short text, no truncation
        assert_eq!(truncate_text("Short.", TruncationPolicy::Sentence(20)), "Short.");

        // No boundary found, should return the full truncated string
        let no_boundary = "This has no boundaries at all and is quite long";
        let result = truncate_text(no_boundary, TruncationPolicy::Sentence(10));
        assert_eq!(result, "This has n (truncated)");
    }

    #[test]
    fn test_truncate_text_sentence_utf8() {
        let text = "🦀 is a crab. 🦀 is cool!";
        // "🦀 is a crab." is 13 chars (🦀 is 1 char in chars() but 4 bytes)
        // Let's truncate at 15 chars.
        let result = truncate_text(text, TruncationPolicy::Sentence(15));
        assert_eq!(result, "🦀 is a crab. (truncated)");
    }
}

pub(crate) fn truncate_text(text: &str, policy: TruncationPolicy) -> String {
    if text.is_empty() {
        return String::new();
    }

    match policy {
        TruncationPolicy::Head(n) => {
            let count = text.chars().count();
            if count <= n {
                text.to_string()
            } else {
                format!("{}... [truncated, {} total chars]", text.chars().take(n).collect::<String>(), count)
            }
        }
        TruncationPolicy::Tail(n) => {
            if text.len() <= n {
                text.to_string()
            } else {
                let skip = text.chars().count().saturating_sub(n);
                format!("[... truncated {} chars] {}", skip, text.chars().skip(skip).collect::<String>())
            }
        }
        TruncationPolicy::HeadAndTail(head, tail) => {
            let total_chars = text.chars().count();
            if total_chars <= head + tail + 50 {
                text.to_string()
            } else {
                let head_text: String = text.chars().take(head).collect();
                let tail_text: String = text.chars().skip(total_chars - tail).collect();
                format!("{}... [{} chars omitted] ...{}", head_text, total_chars - head - tail, tail_text)
            }
        }
        TruncationPolicy::Lines(n) => {
            let lines: Vec<&str> = text.lines().collect();
            if lines.len() <= n {
                text.to_string()
            } else {
                let head_lines = lines[..n].join("\n");
                format!("{}\n\n[truncated, {} additional lines omitted]", head_lines, lines.len() - n)
            }
        }
        TruncationPolicy::Sentence(n) => {
            if text.chars().count() <= n {
                text.to_string()
            } else {
                // Truncate at n using char indices to avoid UTF-8 boundary issues
                let truncated: String = text.chars().take(n).collect();

                // Find last sentence boundary (., !, ?, or newline)
                let last_boundary = truncated
                    .char_indices()
                    .rfind(|(_, c)| matches!(c, '.' | '!' | '?' | '\n'));

                let result = match last_boundary {
                    Some((pos, '\n')) => truncated[..pos].to_string(),
                    Some((pos, _)) => truncated[..=pos].to_string(),
                    None => truncated.to_string(),
                };

                format!("{} (truncated)", result.trim())
            }
        }
    }
}
