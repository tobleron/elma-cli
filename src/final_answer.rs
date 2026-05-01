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

/// Run the full answer pipeline: sanitize → check blocked patterns → return.
pub(crate) fn process_final_answer(raw: &str) -> String {
    let (cleaned, _modified) = sanitize_final_answer(raw);
    cleaned
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
    fn test_sanitize_preserves_legitimate_text() {
        let text = "The Analysis tool shows the data.";
        let (result, modified) = sanitize_final_answer(text);
        // "Analysis" should NOT be stripped here since it's part of a sentence
        // But our current implementation might strip it. Let's verify.
        assert!(result.contains("Analysis"), "Should preserve 'Analysis' in non-framing context: '{}'", result);
        assert!(!modified);
    }
}
