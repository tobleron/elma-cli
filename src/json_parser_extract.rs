//! @efficiency-role: util-pure
//!
//! JSON Parser - Extraction Helpers
//!
//! Extraction and field-parsing utilities used by the core json_parser module.

use crate::json_parser::{extract_json_object, parse_intel_output};
use crate::*;
use std::any::TypeId;

/// Extract JSON from markdown code blocks
pub(crate) fn extract_from_markdown(text: &str) -> Option<String> {
    // Pattern: ```json ... ``` or ``` ... ```
    let start = text.find("```")?;
    let after_start = &text[start + 3..];

    // Skip optional language identifier (json, JSON, etc.)
    let content_start = if after_start.trim_start().starts_with("json") {
        let json_end = after_start.find('\n').unwrap_or(after_start.len());
        &after_start[json_end..]
    } else {
        after_start
    };

    let end = content_start.find("```")?;
    Some(content_start[..end].trim().to_string())
}

/// Extract JSON object from text by finding balanced braces
pub(crate) fn extract_json_from_text(text: &str) -> Option<String> {
    let start = text.find('{')?;
    let mut depth = 0;
    let mut in_string = false;
    let mut escape_next = false;

    for (i, c) in text[start..].char_indices() {
        if escape_next {
            escape_next = false;
            continue;
        }

        match c {
            '\\' if in_string => escape_next = true,
            '"' => in_string = !in_string,
            '{' if !in_string => depth += 1,
            '}' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    return Some(text[start..start + i + 1].to_string());
                }
            }
            _ => {}
        }
    }

    None
}

/// Placeholder for extract_json_from_markdown_wrapped (not yet implemented)
pub(crate) fn extract_json_from_markdown_wrapped(_text: &str) -> Option<String> {
    None
}

/// Placeholder for extract_json_from_pure_json (not yet implemented)
pub(crate) fn extract_json_from_pure_json(_text: &str) -> Option<String> {
    None
}

/// Placeholder for extract_json_with_prose_after (not yet implemented)
pub(crate) fn extract_json_with_prose_after(_text: &str) -> Option<String> {
    None
}

/// Placeholder for fix_orphaned_keys_in_arrays (not yet implemented)
pub(crate) fn fix_orphaned_keys_in_arrays(_text: &str) -> Option<String> {
    None
}

fn parse_string_field(raw: &str, field: &str) -> Option<String> {
    let quoted = format!("\"{}\"", field);
    let key_pos = raw
        .find(&quoted)
        .or_else(|| raw.find(field))
        .map(|pos| pos + field.len())?;
    let after_key = &raw[key_pos..];
    let colon = after_key.find(':')?;
    let mut value = after_key[colon + 1..].trim_start();

    if let Some(rest) = value.strip_prefix('"') {
        let mut escaped = false;
        let mut out = String::new();
        for ch in rest.chars() {
            if escaped {
                out.push(ch);
                escaped = false;
                continue;
            }
            match ch {
                '\\' => escaped = true,
                '"' => return Some(out),
                _ => out.push(ch),
            }
        }
        return None;
    }

    let end = value
        .find(|c: char| c == ',' || c == '}' || c.is_whitespace())
        .unwrap_or(value.len());
    value = &value[..end];
    let trimmed = value.trim_matches('"').trim_matches('\'').trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn parse_number_field(raw: &str, field: &str) -> Option<f64> {
    let quoted = format!("\"{}\"", field);
    let key_pos = raw
        .find(&quoted)
        .or_else(|| raw.find(field))
        .map(|pos| pos + field.len())?;
    let after_key = &raw[key_pos..];
    let colon = after_key.find(':')?;
    let value = after_key[colon + 1..].trim_start();
    let end = value
        .find(|c: char| c == ',' || c == '}' || c.is_whitespace())
        .unwrap_or(value.len());
    value[..end].trim().parse::<f64>().ok()
}

fn parse_bool_field(raw: &str, field: &str) -> Option<bool> {
    match parse_string_field(raw, field)?
        .to_ascii_lowercase()
        .as_str()
    {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn build_scope_fallback(raw: &str) -> serde_json::Value {
    let objective = parse_string_field(raw, "objective")
        .unwrap_or_else(|| "Complete the user's request".to_string());
    let reason = parse_string_field(raw, "reason")
        .unwrap_or_else(|| "Fallback scope after parse repair".to_string());
    serde_json::json!({
        "objective": objective,
        "focus_paths": [],
        "include_globs": [],
        "exclude_globs": [],
        "query_terms": [],
        "expected_artifacts": [],
        "reason": reason,
    })
}

fn build_formula_fallback(raw: &str) -> serde_json::Value {
    serde_json::json!({
        "primary": parse_string_field(raw, "primary").unwrap_or_else(|| "reply_only".to_string()),
        "alternatives": [],
        "reason": parse_string_field(raw, "reason").unwrap_or_else(|| "Fallback formula after parse repair".to_string()),
        "memory_id": parse_string_field(raw, "memory_id").unwrap_or_default(),
    })
}

fn build_workflow_fallback(raw: &str) -> serde_json::Value {
    let scope = build_scope_fallback(raw);
    serde_json::json!({
        "objective": parse_string_field(raw, "objective").unwrap_or_else(|| "Complete the user's request".to_string()),
        "complexity": parse_string_field(raw, "complexity").unwrap_or_else(|| "DIRECT".to_string()),
        "risk": parse_string_field(raw, "risk").unwrap_or_else(|| "LOW".to_string()),
        "needs_evidence": parse_bool_field(raw, "needs_evidence").unwrap_or(false),
        "scope": scope,
        "preferred_formula": parse_string_field(raw, "preferred_formula").unwrap_or_else(|| "reply_only".to_string()),
        "alternatives": [],
        "memory_id": parse_string_field(raw, "memory_id").unwrap_or_default(),
        "reason": parse_string_field(raw, "reason").unwrap_or_else(|| "Fallback workflow after parse repair".to_string()),
    })
}

fn build_critic_like_fallback(raw: &str, default_status: &str) -> serde_json::Value {
    serde_json::json!({
        "status": parse_string_field(raw, "status").unwrap_or_else(|| default_status.to_string()),
        "reason": parse_string_field(raw, "reason").unwrap_or_else(|| "Fallback verdict after parse repair".to_string()),
    })
}

fn build_classification_fallback(raw: &str) -> serde_json::Value {
    let parsed = parse_intel_output(raw, &[("0", "UNKNOWN")]);
    let choice = parsed
        .choice
        .or(parsed.label.clone())
        .unwrap_or_else(|| "UNKNOWN".to_string());
    let label = parsed.label.unwrap_or_else(|| choice.clone());
    serde_json::json!({
        "choice": choice,
        "label": label,
        "reason": parsed.reason.unwrap_or_else(|| "Fallback classification after parse repair".to_string()),
        "entropy": parsed.entropy.unwrap_or(0.5),
    })
}

pub(crate) fn regex_fallback_value<T: 'static>(raw: &str) -> Option<serde_json::Value> {
    let type_id = TypeId::of::<T>();

    if type_id == TypeId::of::<ScopePlan>() {
        Some(build_scope_fallback(raw))
    } else if type_id == TypeId::of::<FormulaSelection>() {
        Some(build_formula_fallback(raw))
    } else if type_id == TypeId::of::<WorkflowPlannerOutput>() {
        Some(build_workflow_fallback(raw))
    } else if type_id == TypeId::of::<CriticVerdict>()
        || type_id == TypeId::of::<OutcomeVerificationVerdict>()
        || type_id == TypeId::of::<ExecutionSufficiencyVerdict>()
        || type_id == TypeId::of::<RepairSemanticsVerdict>()
    {
        Some(build_critic_like_fallback(raw, "ok"))
    } else if type_id == TypeId::of::<RiskReviewVerdict>() {
        Some(build_critic_like_fallback(raw, "caution"))
    } else if type_id == TypeId::of::<ClaimCheckVerdict>() {
        Some(serde_json::json!({
            "status": parse_string_field(raw, "status").unwrap_or_else(|| "ok".to_string()),
            "reason": parse_string_field(raw, "reason").unwrap_or_else(|| "Fallback claim-check verdict after parse repair".to_string()),
            "unsupported_claims": [],
            "missing_points": [],
            "rewrite_instructions": parse_string_field(raw, "rewrite_instructions").unwrap_or_default(),
        }))
    } else if type_id == TypeId::of::<serde_json::Value>() {
        Some(build_classification_fallback(raw))
    } else {
        None
    }
}

/// Extract command from intel unit output (for command_repair)
pub(crate) fn extract_cmd(raw: &str) -> Option<String> {
    let raw_trimmed = raw.trim();
    extract_json_object(raw_trimmed).and_then(|json| {
        json.get("cmd")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    })
}

/// Extract entropy from intel unit output
pub(crate) fn extract_entropy(raw: &str) -> Option<f64> {
    let raw_trimmed = raw.trim();
    extract_json_object(raw_trimmed).and_then(|json| json.get("entropy").and_then(|v| v.as_f64()))
}

/// Extract reason from intel unit output
pub(crate) fn extract_reason(raw: &str) -> String {
    let raw_trimmed = raw.trim();
    extract_json_object(raw_trimmed)
        .and_then(|json| {
            json.get("reason")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_default()
}

/// Extract label from intel unit output (backward compatible wrapper)
pub(crate) fn extract_label(
    raw: &str,
    pairs: &'static [(&'static str, &'static str)],
) -> Option<&'static str> {
    let result = super::parse_intel_output(raw, pairs);
    result.label.and_then(|label| {
        pairs
            .iter()
            .find(|(_, l)| l.eq_ignore_ascii_case(&label))
            .map(|(_, l)| *l)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_PAIRS: &[(&str, &str)] = &[("1", "CHAT"), ("2", "WORKFLOW"), ("3", "INQUIRE")];

    #[test]
    fn test_extract_entropy_direct() {
        let raw = r#"{"choice": "1", "entropy": 0.75}"#;
        assert_eq!(extract_entropy(raw), Some(0.75));
    }

    #[test]
    fn test_extract_entropy_markdown() {
        let raw = r#"```{"entropy": 0.5}```"#;
        assert_eq!(extract_entropy(raw), Some(0.5));
    }

    #[test]
    fn test_extract_entropy_missing() {
        let raw = r#"{"choice": "1"}"#;
        assert_eq!(extract_entropy(raw), None);
    }

    #[test]
    fn test_extract_reason() {
        let raw = r#"{"reason": "user is greeting"}"#;
        assert_eq!(extract_reason(raw), "user is greeting");
    }

    #[test]
    fn test_extract_reason_empty() {
        let raw = r#"{"choice": "1"}"#;
        assert_eq!(extract_reason(raw), "");
    }

    #[test]
    fn test_extract_cmd() {
        let raw = r#"{"cmd": "rg --hidden foo", "reason": "fixed glob"}"#;
        assert_eq!(extract_cmd(raw), Some("rg --hidden foo".to_string()));
    }

    #[test]
    fn test_case_insensitive_label_match() {
        let raw = r#"{"label": "chat"}"#;
        let result = parse_intel_output(raw, TEST_PAIRS);
        assert_eq!(result.label, Some("chat".to_string()));
        assert_eq!(extract_label(raw, TEST_PAIRS), Some("CHAT"));
    }
}
