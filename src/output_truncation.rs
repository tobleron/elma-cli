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
}

impl Default for TruncationPolicy {
    fn default() -> Self {
        TruncationPolicy::Head(50_000)
    }
}

pub(crate) fn truncate_text(text: &str, policy: TruncationPolicy) -> String {
    if text.is_empty() {
        return String::new();
    }

    match policy {
        TruncationPolicy::Head(n) => {
            if text.len() <= n {
                text.to_string()
            } else {
                format!("{}... [truncated, {} total chars]", text.chars().take(n).collect::<String>(), text.len())
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
    }
}
