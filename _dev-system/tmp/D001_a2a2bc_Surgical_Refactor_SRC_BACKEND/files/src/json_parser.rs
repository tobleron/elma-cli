//! @efficiency-role: util-pure
//!
//! JSON Parser - Core parsing logic for intel unit outputs
//!
//! This module provides fault-tolerant JSON parsing for intel unit outputs.
//! It handles common model output issues:
//! - Markdown code blocks (```json ... ```)
//! - Leading/trailing text ("Here's my answer: {...}")
//! - Truncated JSON
//! - Malformed JSON with jsonrepair-rs repair attempts
//! - Legacy non-JSON formats

use crate::json_parser_extract;
use crate::*;
use jsonrepair_rs::jsonrepair;
use std::any::TypeId;

pub(crate) use json_parser_extract::{
    extract_cmd, extract_entropy, extract_label, extract_reason, regex_fallback_value,
};

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
pub(crate) fn extract_json_object(raw: &str) -> Option<serde_json::Value> {
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
    if let Some(json_str) = json_parser_extract::extract_from_markdown(raw_trimmed) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&json_str) {
            if json.is_object() {
                return Some(json);
            }
        }
    }

    // Try finding JSON object in text (look for first '{' and last '}')
    if let Some(json_str) = json_parser_extract::extract_json_from_text(raw_trimmed) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&json_str) {
            if json.is_object() {
                return Some(json);
            }
        }
    } else {
        // If balanced extraction failed, try repairing the text from first '{' to end
        if let Some(start) = raw_trimmed.find('{') {
            let truncated = &raw_trimmed[start..];
            if let Ok(repaired_str) = jsonrepair(truncated) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&repaired_str) {
                    if json.is_object() {
                        return Some(json);
                    }
                }
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

    if let Some(markdown) = json_parser_extract::extract_from_markdown(raw_trimmed) {
        if let Ok(value) = try_parse_candidate::<T>(&markdown) {
            return Ok(value);
        }
    }

    if let Some(extracted) = json_parser_extract::extract_json_from_text(raw_trimmed) {
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
