//! @efficiency-role: domain-logic
//!
//! Footer Contract — validates the status bar only shows core runtime metrics.
//!
//! Task 641: Footer contract that enforces Rule 5 of Elma's philosophy:
//! the bottom status bar must show only: model name, token count, elapsed time.
//! Execution mode, queue notices, operational notifications, and routing
//! decisions belong in the transcript, not the footer.

use crate::*;

// ============================================================================
// FooterData — allowed core runtime metrics only
// ============================================================================

/// Pure data for the footer: only model name, token count, and elapsed time.
/// This enforces the contract by limiting what can be rendered.
#[derive(Clone, Debug, Default)]
pub(crate) struct FooterData {
    pub model: Option<String>,
    pub tokens: u64,
    pub elapsed_secs: u64,
}

// ============================================================================
// FooterContract — validates footer text against the contract
// ============================================================================

/// Validates that a footer string respects the core-metrics-only rule.
#[derive(Clone, Debug)]
pub(crate) struct FooterContract;

impl FooterContract {
    /// Validate a footer line against the contract.
    /// Returns a list of violation descriptions (empty = compliant).
    pub(crate) fn validate(footer_text: &str) -> Vec<String> {
        let mut violations: Vec<String> = Vec::new();

        let lower = footer_text.to_lowercase();

        // Check for forbidden metrics that belong in the transcript
        if lower.contains("mode") {
            violations.push(
                "Footer contains 'mode' — execution mode belongs in the transcript".to_string(),
            );
        }
        if lower.contains("queue") {
            violations.push(
                "Footer contains 'queue' — queue notices belong in the transcript".to_string(),
            );
        }
        if lower.contains("transcript_metric") {
            violations.push(
                "Footer contains 'transcript_metric' — routing info belongs in the transcript"
                    .to_string(),
            );
        }

        // Check for forbidden substrings (bare words, not inside other words)
        // Re-check with word-boundary awareness if the simple check is too broad.
        // The current check is intentionally permissive to catch violations.

        // Check that at least one core metric is present
        let has_model = lower.contains("model")
            || lower.contains("llm")
            || lower.contains("gpt")
            || lower.contains("claude")
            || lower.contains("gemini")
            || lower.contains("llama");
        let has_tokens = lower.contains("token");
        let has_elapsed =
            lower.contains("elapsed") || lower.contains("sec") || lower.contains("time");

        if !has_model && !has_tokens && !has_elapsed {
            violations.push(
                "Footer missing required metrics — must include at least one of: model name, token count, elapsed time"
                    .to_string(),
            );
        }

        violations
    }
}

// ============================================================================
// Renderer
// ============================================================================

/// Render a simple footer line from structured data.
/// Truncates the model label if the line exceeds the given width.
pub(crate) fn render_footer_from_data(data: &FooterData, width: usize) -> String {
    let model_label = data.model.as_deref().unwrap_or("").to_string();
    let tokens_part = format!("tokens:{}", data.tokens);
    let elapsed_part = format!("{}s", data.elapsed_secs);

    // First try full render
    let full = build_footer_line(&model_label, &tokens_part, &elapsed_part);
    if crate::ui::ui_wrap::display_width(&full) <= width {
        return full;
    }

    // Try with truncated model label
    if !model_label.is_empty() {
        let overflow = crate::ui::ui_wrap::display_width(&full).saturating_sub(width);
        let model_w = crate::ui::ui_wrap::display_width(&model_label);
        if model_w > overflow + 3 {
            let keep = model_w.saturating_sub(overflow + 1);
            let truncated: String = model_label.chars().take(keep).collect();
            let truncated = format!("{}…", truncated);
            let result = build_footer_line(&truncated, &tokens_part, &elapsed_part);
            if crate::ui::ui_wrap::display_width(&result) <= width {
                return result;
            }
        }
    }

    // Next try: tokens + elapsed only
    let minimal = format!("{} | {}", tokens_part, elapsed_part);
    if crate::ui::ui_wrap::display_width(&minimal) <= width {
        return minimal;
    }

    // Next: elapsed only
    if crate::ui::ui_wrap::display_width(&elapsed_part) <= width {
        return elapsed_part;
    }

    // Last resort: truncated elapsed
    if width >= 2 {
        let keep = width.saturating_sub(1);
        let truncated: String = elapsed_part.chars().take(keep).collect();
        return format!("{}…", truncated);
    }

    String::new()
}

fn build_footer_line(model: &str, tokens: &str, elapsed: &str) -> String {
    if model.is_empty() {
        format!("{} | {}", tokens, elapsed)
    } else {
        format!("{} | {} | {}", model, tokens, elapsed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── validate ──────────────────────────────────────────────────────────

    #[test]
    fn test_validate_ok_with_model() {
        let violations = FooterContract::validate("claude-sonnet | tokens:1234 | 45s");
        assert!(
            violations.is_empty(),
            "expected no violations, got: {:?}",
            violations
        );
    }

    #[test]
    fn test_validate_ok_with_tokens_only() {
        let violations = FooterContract::validate("tokens:5678 | 120s");
        assert!(
            violations.is_empty(),
            "expected no violations, got: {:?}",
            violations
        );
    }

    #[test]
    fn test_validate_ok_with_elapsed_only() {
        let violations = FooterContract::validate("elapsed: 99s");
        assert!(
            violations.is_empty(),
            "expected no violations, got: {:?}",
            violations
        );
    }

    #[test]
    fn test_validate_fails_no_metrics() {
        let violations = FooterContract::validate("hello world");
        assert!(!violations.is_empty(), "expected violations for no metrics");
        assert!(violations
            .iter()
            .any(|v| v.contains("missing required metrics")));
    }

    #[test]
    fn test_validate_fails_forbidden_mode() {
        let violations = FooterContract::validate("mode:chat | tokens:100");
        assert!(
            !violations.is_empty(),
            "expected violations for containing mode"
        );
        assert!(violations.iter().any(|v| v.contains("mode")));
    }

    #[test]
    fn test_validate_fails_forbidden_queue() {
        let violations = FooterContract::validate("queue:3 waiting | tokens:100 | 10s");
        assert!(
            !violations.is_empty(),
            "expected violations for containing queue"
        );
        assert!(violations.iter().any(|v| v.contains("queue")));
    }

    #[test]
    fn test_validate_fails_forbidden_transcript_metric() {
        let violations = FooterContract::validate("transcript_metric:0.92 | tokens:100 | 10s");
        assert!(
            !violations.is_empty(),
            "expected violations for transcript_metric"
        );
        assert!(violations.iter().any(|v| v.contains("transcript_metric")));
    }

    #[test]
    fn test_validate_multiple_violations() {
        let violations = FooterContract::validate("mode:auto queue:5");
        // Should flag mode, queue, and missing required metrics
        assert!(
            violations.len() >= 2,
            "expected multiple violations, got: {:?}",
            violations
        );
    }

    #[test]
    fn test_validate_empty_string() {
        let violations = FooterContract::validate("");
        assert!(
            !violations.is_empty(),
            "empty string should have violations"
        );
    }

    // ── render_footer_from_data ───────────────────────────────────────────

    #[test]
    fn test_render_full() {
        let data = FooterData {
            model: Some("claude-sonnet-4".to_string()),
            tokens: 1234,
            elapsed_secs: 45,
        };
        let result = render_footer_from_data(&data, 120);
        assert!(result.contains("claude-sonnet-4"));
        assert!(result.contains("tokens:1234"));
        assert!(result.contains("45s"));
    }

    #[test]
    fn test_render_no_model() {
        let data = FooterData {
            model: None,
            tokens: 500,
            elapsed_secs: 10,
        };
        let result = render_footer_from_data(&data, 120);
        assert_eq!(result, "tokens:500 | 10s");
    }

    #[test]
    fn test_render_zero_tokens() {
        let data = FooterData {
            model: Some("model-x".to_string()),
            tokens: 0,
            elapsed_secs: 0,
        };
        let result = render_footer_from_data(&data, 120);
        assert!(result.contains("tokens:0"));
        assert!(result.contains("0s"));
    }

    #[test]
    fn test_render_truncates_model_when_narrow() {
        let data = FooterData {
            model: Some("very-long-model-name-that-should-be-truncated".to_string()),
            tokens: 999,
            elapsed_secs: 5,
        };
        let result = render_footer_from_data(&data, 30);
        // Should be truncated
        assert!(
            result.len() <= 33,
            "result too long: '{}' ({} chars)",
            result,
            result.len()
        );
    }

    #[test]
    fn test_render_extremely_narrow() {
        let data = FooterData {
            model: Some("big-model".to_string()),
            tokens: 99999,
            elapsed_secs: 999,
        };
        let result = render_footer_from_data(&data, 5);
        // Should not panic; returns truncated or empty
        assert!(result.len() <= 8);
    }

    #[test]
    fn test_render_very_wide() {
        let data = FooterData {
            model: Some("my-model".to_string()),
            tokens: 42,
            elapsed_secs: 7,
        };
        let result = render_footer_from_data(&data, 200);
        assert!(result.contains("my-model"));
        assert!(result.contains("tokens:42"));
        assert!(result.contains("7s"));
    }

    // ── validate + render round-trip ──────────────────────────────────────

    #[test]
    fn test_rendered_passes_validation() {
        let data = FooterData {
            model: Some("gpt-4o".to_string()),
            tokens: 1500,
            elapsed_secs: 60,
        };
        let rendered = render_footer_from_data(&data, 80);
        let violations = FooterContract::validate(&rendered);
        assert!(
            violations.is_empty(),
            "rendered footer should pass validation, got: {:?}",
            violations
        );
    }

    #[test]
    fn test_rendered_no_model_passes_validation() {
        let data = FooterData {
            model: None,
            tokens: 1500,
            elapsed_secs: 60,
        };
        let rendered = render_footer_from_data(&data, 80);
        let violations = FooterContract::validate(&rendered);
        assert!(
            violations.is_empty(),
            "rendered footer (no model) should pass validation, got: {:?}",
            violations
        );
    }
}
