//! @efficiency-role: util-pure
//!
//! JSON Parser - Robust parsing for intel unit outputs
//!
//! This module provides fault-tolerant JSON parsing for intel unit outputs.
//! It handles common model output issues:
//! - Markdown code blocks (```json ... ```)
//! - Leading/trailing text ("Here's my answer: {...}")
//! - Truncated JSON
//! - Malformed JSON with jsonrepair-rs repair attempts
//! - Legacy non-JSON formats

use crate::*;
use jsonrepair_rs::jsonrepair;
use std::any::TypeId;

/// Result of parsing an intel unit output
#[derive(Debug, Clone)]
pub(crate) struct IntelParseResult {
    pub(crate) choice: Option<String>,
    pub(crate) label: Option<String>,
    pub(crate) reason: Option<String>,
    pub(crate) entropy: Option<f64>,
    pub(crate) cmd: Option<String>,
    pub(crate) parse_method: ParseMethod,
}

/// How the parsing succeeded
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ParseMethod {
    /// Clean JSON parsing
    JsonDirect,
    /// JSON extracted from markdown code block
    JsonMarkdown,
    /// JSON extracted from text with leading/trailing content
    JsonExtracted,
    /// Legacy raw token parsing (digit or label)
    LegacyToken,
    /// Failed to parse
    Failed,
}

/// Extract JSON from various output formats
fn extract_json_object(raw: &str) -> Option<serde_json::Value> {
    let raw_trimmed = raw.trim();

    // Try direct JSON parsing first
    if raw_trimmed.starts_with('{') {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(raw_trimmed) {
            if json.is_object() {
                return Some(json);
            }
        }
    }

    // Try extracting JSON from markdown code blocks
    // Matches: ```json {...} ``` or ``` {...} ```
    if let Some(json_str) = extract_from_markdown(raw_trimmed) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&json_str) {
            if json.is_object() {
                return Some(json);
            }
        }
    }

    // Try finding JSON object in text (look for first '{' and last '}')
    if let Some(json_str) = extract_json_from_text(raw_trimmed) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&json_str) {
            if json.is_object() {
                return Some(json);
            }
        }
    }

    // Try repairing malformed JSON using jsonrepair-rs
    // This handles: missing quotes, trailing commas, unescaped newlines, etc.
    if let Ok(repaired_str) = jsonrepair(raw_trimmed) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&repaired_str) {
            if json.is_object() {
                return Some(json);
            }
        }
    }

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

fn regex_fallback_value<T: 'static>(raw: &str) -> Option<serde_json::Value> {
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

pub(crate) fn validate_schema(
    value: &serde_json::Value,
    schema: &JsonSchema,
) -> Result<(), SchemaValidationError> {
    let Some(obj) = value.as_object() else {
        return Err(SchemaValidationError::ValidationErrors(vec![
            "Top-level JSON value must be an object".to_string(),
        ]));
    };

    let mut errors = Vec::new();

    for field in &schema.required_fields {
        if !obj.contains_key(*field) {
            errors.push(format!("Missing required field '{}'", field));
        }
    }

    for (field, field_type) in &schema.field_types {
        if let Some(value) = obj.get(*field) {
            if !field_type.matches(value) {
                errors.push(format!(
                    "Field '{}' must be {}",
                    field,
                    field_type.describe()
                ));
            }
        }
    }

    for validator in &schema.validators {
        if let Some(error) = validator.validate(value) {
            errors.push(error);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(SchemaValidationError::ValidationErrors(errors))
    }
}

fn try_parse_candidate<T: serde::de::DeserializeOwned + 'static>(candidate: &str) -> Result<T> {
    let value = serde_json::from_str::<serde_json::Value>(candidate)?;
    if let Some(schema) = schema_for_type::<T>() {
        validate_schema(&value, &schema)?;
    }
    Ok(serde_json::from_value(value)?)
}

pub(crate) fn parse_with_repair<T: serde::de::DeserializeOwned + 'static>(raw: &str) -> Result<T> {
    let raw_trimmed = raw.trim();

    if let Ok(value) = try_parse_candidate::<T>(raw_trimmed) {
        return Ok(value);
    }

    if let Some(markdown) = extract_from_markdown(raw_trimmed) {
        if let Ok(value) = try_parse_candidate::<T>(&markdown) {
            return Ok(value);
        }
    }

    if let Some(extracted) = extract_json_from_text(raw_trimmed) {
        if let Ok(value) = try_parse_candidate::<T>(&extracted) {
            return Ok(value);
        }
    }

    if let Ok(repaired) = jsonrepair(raw_trimmed) {
        if let Ok(value) = try_parse_candidate::<T>(&repaired) {
            return Ok(value);
        }
    }

    if let Some(fallback) = regex_fallback_value::<T>(raw_trimmed) {
        if let Some(schema) = schema_for_type::<T>() {
            validate_schema(&fallback, &schema)?;
        }
        return Ok(serde_json::from_value(fallback)?);
    }

    anyhow::bail!("Unable to parse JSON after direct parse, extraction, repair, and fallback")
}

/// Extract JSON from markdown code blocks
fn extract_from_markdown(text: &str) -> Option<String> {
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
fn extract_json_from_text(text: &str) -> Option<String> {
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

/// Parse intel unit output and extract all fields
pub(crate) fn parse_intel_output(
    raw: &str,
    pairs: &'static [(&'static str, &'static str)],
) -> IntelParseResult {
    let raw_trimmed = raw.trim();

    // Try JSON parsing (with various extraction methods)
    if let Some(json) = extract_json_object(raw_trimmed) {
        // Determine parse method based on input format
        let parse_method = if raw_trimmed.starts_with("```") {
            ParseMethod::JsonMarkdown
        } else if raw_trimmed.starts_with('{') && raw_trimmed.ends_with('}') {
            // Check if it's clean JSON (starts with { and ends with })
            ParseMethod::JsonDirect
        } else {
            ParseMethod::JsonExtracted
        };

        let mut result = IntelParseResult {
            choice: None,
            label: None,
            reason: None,
            entropy: None,
            cmd: None,
            parse_method,
        };

        // Extract "label" field
        if let Some(label) = json.get("label").and_then(|v| v.as_str()) {
            result.label = Some(label.to_string());
        }

        // Extract "choice" field
        if let Some(choice) = json.get("choice").and_then(|v| v.as_str()) {
            result.choice = Some(choice.to_string());
        }

        // Extract "reason" field
        if let Some(reason) = json.get("reason").and_then(|v| v.as_str()) {
            result.reason = Some(reason.to_string());
        }

        // Extract "entropy" field
        if let Some(entropy) = json.get("entropy").and_then(|v| v.as_f64()) {
            result.entropy = Some(entropy);
        }

        // Extract "cmd" field (for command_repair)
        if let Some(cmd) = json.get("cmd").and_then(|v| v.as_str()) {
            result.cmd = Some(cmd.to_string());
        }

        return result;
    }

    // Legacy fallback: parse raw digit or label
    let token = raw_trimmed
        .trim_matches(|c: char| c == '"' || c == '\'')
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim();

    // Check if token matches a code or label
    for (code, label) in pairs {
        if token == *code || token.eq_ignore_ascii_case(label) {
            return IntelParseResult {
                choice: Some((*code).to_string()),
                label: Some((*label).to_string()),
                reason: None,
                entropy: None,
                cmd: None,
                parse_method: ParseMethod::LegacyToken,
            };
        }
    }

    // Complete failure
    IntelParseResult {
        choice: None,
        label: None,
        reason: None,
        entropy: None,
        cmd: None,
        parse_method: ParseMethod::Failed,
    }
}

/// Extract label from intel unit output (backward compatible wrapper)
pub(crate) fn extract_label(
    raw: &str,
    pairs: &'static [(&'static str, &'static str)],
) -> Option<&'static str> {
    let result = parse_intel_output(raw, pairs);
    result.label.and_then(|label| {
        pairs
            .iter()
            .find(|(_, l)| l.eq_ignore_ascii_case(&label))
            .map(|(_, l)| *l)
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

/// Extract command from intel unit output (for command_repair)
pub(crate) fn extract_cmd(raw: &str) -> Option<String> {
    let raw_trimmed = raw.trim();
    extract_json_object(raw_trimmed).and_then(|json| {
        json.get("cmd")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    })
}

/// Validate entropy is in valid range [0.0, 1.0]
pub(crate) fn validate_entropy(entropy: f64) -> f64 {
    entropy.clamp(0.0, 1.0)
}

/// Get the final label, using fallback if parsing failed
pub(crate) fn get_label_with_fallback(
    result: &IntelParseResult,
    pairs: &'static [(&'static str, &'static str)],
) -> Option<String> {
    result.label.clone().or_else(|| {
        result.choice.as_ref().and_then(|choice| {
            pairs
                .iter()
                .find(|(code, _)| code == &choice)
                .map(|(_, label)| (*label).to_string())
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_PAIRS: &[(&str, &str)] = &[("1", "CHAT"), ("2", "WORKFLOW"), ("3", "INQUIRE")];

    #[test]
    fn test_parse_clean_json() {
        let raw = r#"{"choice": "1", "label": "CHAT", "reason": "greeting", "entropy": 0.2}"#;
        let result = parse_intel_output(raw, TEST_PAIRS);
        assert_eq!(result.choice, Some("1".to_string()));
        assert_eq!(result.label, Some("CHAT".to_string()));
        assert_eq!(result.reason, Some("greeting".to_string()));
        assert_eq!(result.entropy, Some(0.2));
        assert_eq!(result.parse_method, ParseMethod::JsonDirect);
    }

    #[test]
    fn test_parse_json_markdown() {
        let raw = r#"```json
{"choice": "2", "label": "WORKFLOW", "entropy": 0.5}
```"#;
        let result = parse_intel_output(raw, TEST_PAIRS);
        assert_eq!(result.label, Some("WORKFLOW".to_string()));
        assert_eq!(result.entropy, Some(0.5));
        assert_eq!(result.parse_method, ParseMethod::JsonMarkdown);
    }

    #[test]
    fn test_parse_json_with_leading_text() {
        let raw = r#"Here's my classification: {"choice": "1", "label": "CHAT", "entropy": 0.1}"#;
        let result = parse_intel_output(raw, TEST_PAIRS);
        assert_eq!(result.label, Some("CHAT".to_string()));
        assert_eq!(result.parse_method, ParseMethod::JsonExtracted);
    }

    #[test]
    fn test_parse_json_with_trailing_text() {
        let raw = r#"{"choice": "3", "label": "INQUIRE", "entropy": 0.8} Is this correct?"#;
        let result = parse_intel_output(raw, TEST_PAIRS);
        assert_eq!(result.label, Some("INQUIRE".to_string()));
        assert_eq!(result.parse_method, ParseMethod::JsonExtracted);
    }

    #[test]
    fn test_parse_legacy_digit() {
        let raw = "1";
        let result = parse_intel_output(raw, TEST_PAIRS);
        assert_eq!(result.choice, Some("1".to_string()));
        assert_eq!(result.label, Some("CHAT".to_string()));
        assert_eq!(result.parse_method, ParseMethod::LegacyToken);
    }

    #[test]
    fn test_parse_legacy_label() {
        let raw = "WORKFLOW";
        let result = parse_intel_output(raw, TEST_PAIRS);
        assert_eq!(result.label, Some("WORKFLOW".to_string()));
        assert_eq!(result.parse_method, ParseMethod::LegacyToken);
    }

    #[test]
    fn test_parse_quoted_legacy() {
        let raw = r#""CHAT""#;
        let result = parse_intel_output(raw, TEST_PAIRS);
        assert_eq!(result.label, Some("CHAT".to_string()));
        assert_eq!(result.parse_method, ParseMethod::LegacyToken);
    }

    #[test]
    fn test_parse_malformed_json_missing_brace() {
        let raw = r#"{"choice": "1", "label": "CHAT""#;
        let result = parse_intel_output(raw, TEST_PAIRS);
        // jsonrepair-rs may fix this, so we just check it doesn't panic
        // The parse method depends on whether repair succeeded
        assert!(
            result.parse_method == ParseMethod::JsonExtracted
                || result.parse_method == ParseMethod::Failed
        );
    }

    #[test]
    fn test_parse_empty_string() {
        let raw = "";
        let result = parse_intel_output(raw, TEST_PAIRS);
        assert_eq!(result.parse_method, ParseMethod::Failed);
    }

    #[test]
    fn test_parse_whitespace_only() {
        let raw = "   \n\t  ";
        let result = parse_intel_output(raw, TEST_PAIRS);
        assert_eq!(result.parse_method, ParseMethod::Failed);
    }

    #[test]
    fn test_parse_partial_fields() {
        let raw = r#"{"label": "CHAT"}"#;
        let result = parse_intel_output(raw, TEST_PAIRS);
        assert_eq!(result.label, Some("CHAT".to_string()));
        assert_eq!(result.choice, None);
        assert_eq!(result.entropy, None);
    }

    #[test]
    fn test_parse_extra_fields() {
        let raw = r#"{"choice": "1", "label": "CHAT", "entropy": 0.3, "extra": "ignored", "timestamp": 12345}"#;
        let result = parse_intel_output(raw, TEST_PAIRS);
        assert_eq!(result.label, Some("CHAT".to_string()));
        assert_eq!(result.entropy, Some(0.3));
    }

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
    fn test_validate_entropy_clamps() {
        assert_eq!(validate_entropy(-0.5), 0.0);
        assert_eq!(validate_entropy(0.5), 0.5);
        assert_eq!(validate_entropy(1.5), 1.0);
    }

    #[test]
    fn test_get_label_with_fallback_from_choice() {
        let result = IntelParseResult {
            choice: Some("2".to_string()),
            label: None,
            reason: None,
            entropy: None,
            cmd: None,
            parse_method: ParseMethod::JsonDirect,
        };
        assert_eq!(
            get_label_with_fallback(&result, TEST_PAIRS),
            Some("WORKFLOW".to_string())
        );
    }

    #[test]
    fn test_get_label_with_fallback_prefers_label() {
        let result = IntelParseResult {
            choice: Some("1".to_string()),
            label: Some("CHAT".to_string()),
            reason: None,
            entropy: None,
            cmd: None,
            parse_method: ParseMethod::JsonDirect,
        };
        assert_eq!(
            get_label_with_fallback(&result, TEST_PAIRS),
            Some("CHAT".to_string())
        );
    }

    #[test]
    fn test_markdown_with_language_tag() {
        let raw = r#"```json
{"choice": "1", "label": "CHAT"}
```"#;
        let result = parse_intel_output(raw, TEST_PAIRS);
        assert_eq!(result.label, Some("CHAT".to_string()));
        assert_eq!(result.parse_method, ParseMethod::JsonMarkdown);
    }

    #[test]
    fn test_markdown_without_language_tag() {
        let raw = r#"```
{"choice": "2", "label": "WORKFLOW"}
```"#;
        let result = parse_intel_output(raw, TEST_PAIRS);
        assert_eq!(result.label, Some("WORKFLOW".to_string()));
        assert_eq!(result.parse_method, ParseMethod::JsonMarkdown);
    }

    #[test]
    fn test_nested_braces_in_string() {
        let raw = r#"{"reason": "test {nested} braces", "label": "CHAT"}"#;
        let result = parse_intel_output(raw, TEST_PAIRS);
        assert_eq!(result.label, Some("CHAT".to_string()));
        assert_eq!(result.reason, Some("test {nested} braces".to_string()));
    }

    #[test]
    fn test_escaped_quotes_in_string() {
        let raw = r#"{"reason": "user said \"hello\"", "label": "CHAT"}"#;
        let result = parse_intel_output(raw, TEST_PAIRS);
        assert_eq!(result.label, Some("CHAT".to_string()));
        assert_eq!(result.reason, Some("user said \"hello\"".to_string()));
    }

    #[test]
    fn test_case_insensitive_label_match() {
        let raw = r#"{"label": "chat"}"#;
        let result = parse_intel_output(raw, TEST_PAIRS);
        assert_eq!(result.label, Some("chat".to_string()));
        assert_eq!(extract_label(raw, TEST_PAIRS), Some("CHAT"));
    }

    #[test]
    fn test_repair_trailing_comma() {
        // jsonrepair-rs should fix trailing comma
        let raw = r#"{"choice": "1", "label": "CHAT",}"#;
        let result = parse_intel_output(raw, TEST_PAIRS);
        assert_eq!(result.label, Some("CHAT".to_string()));
    }

    #[test]
    fn test_repair_missing_quotes() {
        // jsonrepair-rs should fix unquoted keys/values
        let raw = r#"{choice: "1", label: CHAT}"#;
        let result = parse_intel_output(raw, TEST_PAIRS);
        assert_eq!(result.label, Some("CHAT".to_string()));
    }

    #[test]
    fn test_repair_single_quotes() {
        // jsonrepair-rs should convert single quotes to double
        let raw = r#"{'choice': '1', 'label': 'CHAT'}"#;
        let result = parse_intel_output(raw, TEST_PAIRS);
        assert_eq!(result.label, Some("CHAT".to_string()));
    }
}
