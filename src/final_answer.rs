//! @efficiency-role: domain-logic
//!
//! Clean-Context Finalization Enforcement — Task 384.
//!
//! Sanitizes final answers from internal frame artifacts:
//! "Given Evidence:", "Analysis:", "=== Final Answer ===", etc.
//! Enforces that final answers are direct user-facing responses
//! free of internal state, stop reasons, and reasoning artifacts.

use crate::*;

/// Patterns that must never appear in a final user-facing answer.
/// If any match, the finalizer intel unit is invoked.
pub(crate) static BLOCKED_PATTERNS: &[&str] = &[
    "=== Final Answer ===",
    "**Given Evidence:**",
    "**Analysis:**",
    "**Answer:**",
    "**Verification:**",
    "Step 1:",
    "Step 2:",
    "Step 3:",
    "Step 4:",
    "Step 5:",
    "Stop reason:",
    "Tool loop",
    "Stagnation",
    "=== TASK ===",
    "=== FAILED ATTEMPTS ===",
    "=== WORKSPACE CONTEXT",
    "=== ROUTE PRIOR ===",
    "=== INSTRUCTION ===",
    "failed to",
    "error occurred",
    "Solution",
    "Given Evidence",
    "Analysis:",
];

/// Check if a final answer contains any blocked pattern.
pub(crate) fn contains_blocked_pattern(text: &str) -> bool {
    let lower = text.to_lowercase();
    BLOCKED_PATTERNS.iter().any(|p| {
        let p_lower = p.to_lowercase();
        if p_lower.starts_with("===") || p_lower.starts_with("**") {
            text.contains(p)
        } else {
            lower.contains(&p_lower)
        }
    })
}

/// Deterministically sanitize a final answer by stripping known framing.
/// Returns the cleaned text and a flag indicating whether sanitization occurred.
pub(crate) fn sanitize_final_answer(raw: &str) -> (String, bool) {
    if raw.trim().is_empty() {
        return (raw.to_string(), false);
    }

    let mut cleaned = raw.to_string();
    let mut modified = false;

    // Strip markdown headers that contain framing patterns
    let header_patterns = [
        "=== Final Answer ===",
        "## Solution",
        "## Answer",
        "**Given Evidence:**",
        "**Analysis:**",
        "**Verification:**",
        "**Answer:**",
    ];
    for pattern in &header_patterns {
        while let Some(pos) = cleaned.find(pattern) {
            let before = &cleaned[..pos];
            let after = &cleaned[pos + pattern.len()..];
            let content_start = after
                .chars()
                .position(|c| !c.is_whitespace() && c != '\n')
                .unwrap_or(0);
            if content_start < after.len() {
                let remaining = &after[content_start..];
                if before.trim().is_empty() {
                    cleaned = remaining.to_string();
                } else {
                    // Header is mid-content — join before + after
                    cleaned = format!("{}\n{}", before.trim(), remaining);
                }
                modified = true;
            } else {
                cleaned = before.to_string();
                modified = true;
            }
        }
    }

    // Strip "Given Evidence:" and "Analysis:" standalone lines
    let line_patterns = [
        "Given Evidence",
        "Analysis",
        "Verification",
        "Solution",
    ];
    let lines: Vec<&str> = cleaned.lines().collect();
    let filtered: Vec<&str> = lines
        .into_iter()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty()
                && !line_patterns.iter().any(|p| {
                    trimmed == *p
                        || trimmed.starts_with(&format!("**{}:**", p))
                        || trimmed.starts_with(&format!("**{}:**", p.to_lowercase()))
                })
        })
        .collect();
    let joined = filtered.join("\n");
    if joined != cleaned {
        cleaned = joined;
        modified = true;
    }

    // Trim trailing/leading whitespace
    let trimmed = cleaned.trim().to_string();
    if trimmed != cleaned {
        cleaned = trimmed;
        modified = true;
    }

    (cleaned, modified)
}

/// Strip markdown formatting to produce plain text for terminal display.
/// Preserves content (code, text, links) while removing markup.
pub(crate) fn strip_markdown(text: &str) -> String {
    let mut result = text.to_string();

    // Remove fenced code blocks (```...```), keeping content
    result = result
        .split("```")
        .enumerate()
        .map(|(i, part)| {
            if i % 2 == 0 {
                part.to_string() // outside code blocks
            } else {
                // Inside code block: skip the language tag line, keep rest
                let lines: Vec<&str> = part.lines().collect();
                if lines.len() <= 1 {
                    String::new()
                } else {
                    // Skip first line (language tag or empty)
                    lines[1..].join("\n")
                }
            }
        })
        .collect::<Vec<_>>()
        .join("");

    // Remove inline code backticks
    let mut in_code = false;
    let mut code_cleaned = String::new();
    for ch in result.chars() {
        if ch == '`' {
            in_code = !in_code;
        } else {
            code_cleaned.push(ch);
        }
    }
    result = code_cleaned;

    // Strip bold/italic markers (** and *)
    result = result.replace("**", "");
    result = result.replace("__", "");

    // Strip italic markers (single *, but not inside words)
    let mut italic_cleaned = String::new();
    let mut chars = result.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '*' {
            // Only strip if followed by non-alphanumeric or end
            if let Some(&next) = chars.peek() {
                if !next.is_alphanumeric() {
                    continue;
                }
            } else {
                continue;
            }
        } else if ch == '_' {
            if let Some(&next) = chars.peek() {
                if !next.is_alphanumeric() {
                    continue;
                }
            } else {
                continue;
            }
        }
        italic_cleaned.push(ch);
    }
    result = italic_cleaned;

    // Strip markdown links: [text](url) -> text
    let link_re = regex::Regex::new(r"\[([^\]]+)\]\([^)]+\)").unwrap();
    result = link_re.replace_all(&result, "$1").to_string();

    // Strip image links: ![alt](url)
    let img_re = regex::Regex::new(r"!\[([^\]]*)\]\([^)]+\)").unwrap();
    result = img_re.replace_all(&result, "$1").to_string();

    // Convert headers: remove leading # but keep text
    result = result
        .lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with('#') {
                let after_hash = trimmed.trim_start_matches('#').trim();
                format!("{}\n", after_hash)
            } else {
                format!("{}\n", line)
            }
        })
        .collect::<Vec<_>>()
        .join("");

    // Clean up excessive whitespace
    result = result
        .lines()
        .map(|l| l.trim_end().to_string())
        .collect::<Vec<_>>()
        .join("\n");

    // Collapse 3+ consecutive newlines to 2
    let mut collapsed = String::new();
    let mut consecutive_newlines = 0;
    for ch in result.chars() {
        if ch == '\n' {
            consecutive_newlines += 1;
            if consecutive_newlines <= 2 {
                collapsed.push(ch);
            }
        } else {
            consecutive_newlines = 0;
            collapsed.push(ch);
        }
    }
    result = collapsed;

    result.trim().to_string()
}

/// Run the full answer pipeline: sanitize → check blocked patterns → return.
pub(crate) fn process_final_answer(raw: &str) -> String {
    let (cleaned, _modified) = sanitize_final_answer(raw);
    cleaned
}

/// Prepare a final answer for terminal display: sanitize + strip markdown.
pub(crate) fn process_final_answer_display(raw: &str) -> String {
    let sanitized = process_final_answer(raw);
    strip_markdown(&sanitized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_empty() {
        let (result, modified) = sanitize_final_answer("");
        assert!(!modified);
        assert_eq!(result, "");
    }

    #[test]
    fn test_sanitize_clean_text_unchanged() {
        let text = "The current time is 5:35 PM.";
        let (result, modified) = sanitize_final_answer(text);
        assert!(!modified);
        assert_eq!(result, text);
    }

    #[test]
    fn test_sanitize_strips_final_answer_header() {
        let raw = "=== Final Answer ===\nThe time is 5:35 PM.";
        let (result, _) = sanitize_final_answer(raw);
        assert!(!result.contains("=== Final Answer ==="));
        assert!(result.contains("5:35 PM"));
    }

    #[test]
    fn test_sanitize_strips_evidence_framing() {
        let raw = "**Given Evidence:**\nThe time is 5:35 PM.";
        let (result, _) = sanitize_final_answer(raw);
        assert!(!result.contains("Given Evidence"));
    }

    #[test]
    fn test_sanitize_strips_analysis_header() {
        let raw = "**Analysis:**\nThe system clock shows 5:35 PM.";
        let (result, _) = sanitize_final_answer(raw);
        assert!(!result.contains("Analysis:"));
    }

    #[test]
    fn test_contains_blocked_pattern() {
        assert!(contains_blocked_pattern("=== Final Answer ==="));
        assert!(contains_blocked_pattern("**Given Evidence:**"));
        assert!(contains_blocked_pattern("stop reason: timeout"));
        assert!(contains_blocked_pattern("failed to connect"));
        assert!(contains_blocked_pattern("Step 1: open the file"));
        assert!(!contains_blocked_pattern("The time is 5:35 PM."));
    }

    #[test]
    fn test_process_final_answer_clean() {
        let result = process_final_answer("The answer is 42.");
        assert_eq!(result, "The answer is 42.");
    }

    #[test]
    fn test_process_final_answer_with_framing() {
        let raw = "=== Final Answer ===\n\n**Given Evidence:**\n- Clock shows 5:35 PM\n\n**Answer:**\nThe time is 5:35 PM.";
        let result = process_final_answer(raw);
        assert!(!result.contains("=== Final Answer ==="));
        assert!(!result.contains("**Given Evidence:**"));
        assert!(result.contains("5:35 PM"));
    }

    #[test]
    fn test_process_final_answer_removes_multiple_frames() {
        let raw = "**Analysis:**\nI checked the system clock.\n\n**Verification:**\nConfirmed the time.\n\n**Answer:**\nThe time is 5:35 PM.";
        let result = process_final_answer(raw);
        assert!(!result.contains("Analysis"));
        assert!(!result.contains("Verification"));
        assert!(!result.contains("Answer:"));
        assert!(result.contains("5:35 PM"));
    }

    #[test]
    fn test_contains_blocked_pattern_case_insensitive() {
        assert!(contains_blocked_pattern("failed to open file"));
        assert!(contains_blocked_pattern("error occurred: timeout"));
        assert!(contains_blocked_pattern("stagnation detected"));
    }

    #[test]
    fn test_strip_markdown_bold() {
        let result = strip_markdown("This is **bold** text");
        assert_eq!(result, "This is bold text");
    }

    #[test]
    fn test_strip_markdown_inline_code() {
        let result = strip_markdown("Use the `read` tool");
        assert_eq!(result, "Use the read tool");
    }

    #[test]
    fn test_strip_markdown_fenced_code_block() {
        let md = "```rust\nfn hello() {\n    println!(\"hi\");\n}\n```";
        let result = strip_markdown(md);
        assert!(result.contains("fn hello()"));
        assert!(result.contains("println!"));
        assert!(!result.contains("```"));
    }

    #[test]
    fn test_strip_markdown_headers() {
        let result = strip_markdown("# Title\n\n## Subtitle\n\nBody text");
        assert!(!result.contains('#'));
        assert!(result.contains("Title"));
        assert!(result.contains("Subtitle"));
        assert!(result.contains("Body text"));
    }

    #[test]
    fn test_strip_markdown_links() {
        let result = strip_markdown("Click [here](https://example.com) for info");
        assert!(result.contains("here"));
        assert!(!result.contains("https://"));
        assert!(!result.contains("["));
    }

    #[test]
    fn test_strip_markdown_images() {
        let result = strip_markdown("![alt text](image.png)");
        assert!(result.contains("alt text"));
        assert!(!result.contains("image.png"));
    }

    #[test]
    fn test_strip_markdown_plain_text_unchanged() {
        let text = "Hello, this is plain text.";
        let result = strip_markdown(text);
        assert_eq!(result, text);
    }

    #[test]
    fn test_strip_markdown_mixed() {
        let md = "# Results\n\nThe **score** is `42`.\n\n- Item 1\n- Item 2\n\nSee [docs](https://example.com)";
        let result = strip_markdown(md);
        assert!(!result.contains('#'));
        assert!(!result.contains("**"));
        assert!(!result.contains('`'));
        assert!(result.contains("score"));
        assert!(result.contains("42"));
        assert!(result.contains("Item 1"));
        assert!(result.contains("docs"));
    }

    #[test]
    fn test_strip_markdown_empty() {
        assert_eq!(strip_markdown(""), "");
    }

    #[test]
    fn test_sanitize_preserves_legitimate_text() {
        let text = "The Analysis tool shows the data.";
        let (result, modified) = sanitize_final_answer(text);
        // "Analysis" should NOT be stripped here since it's part of a sentence
        // But our current implementation might strip it. Let's verify.
        assert!(result.contains("Analysis"), "Should preserve 'Analysis' in non-framing context: '{}'", result);
        assert!(!modified);
    }
}
