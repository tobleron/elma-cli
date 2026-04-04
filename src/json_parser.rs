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
    JsonDirect,
    JsonMarkdown,
    JsonExtracted,
    LegacyToken,
    Failed,
}

fn try_json_parse(s: &str) -> Option<serde_json::Value> {
    serde_json::from_str::<serde_json::Value>(s)
        .ok()
        .filter(|j| j.is_object())
}

fn try_repair_and_parse(s: &str) -> Option<serde_json::Value> {
    jsonrepair(s)
        .ok()
        .and_then(|repaired| try_json_parse(&repaired))
}

fn json_field(json: &serde_json::Value, key: &str) -> Option<String> {
    json.get(key).and_then(|v| v.as_str()).map(str::to_string)
}

fn json_field_f64(json: &serde_json::Value, key: &str) -> Option<f64> {
    json.get(key).and_then(|v| v.as_f64())
}

pub(crate) fn extract_json_object(raw: &str) -> Option<serde_json::Value> {
    let raw_trimmed = raw.trim();
    if raw_trimmed.starts_with('{') {
        if let Some(json) = try_json_parse(raw_trimmed) {
            return Some(json);
        }
    }
    if let Some(json_str) = json_parser_extract::extract_from_markdown(raw_trimmed) {
        if let Some(json) = try_json_parse(&json_str) {
            return Some(json);
        }
    }
    if let Some(json_str) = json_parser_extract::extract_json_from_text(raw_trimmed) {
        if let Some(json) = try_json_parse(&json_str) {
            return Some(json);
        }
    } else if let Some(start) = raw_trimmed.find('{') {
        if let Some(json) = try_repair_and_parse(&raw_trimmed[start..]) {
            return Some(json);
        }
    }
    try_repair_and_parse(raw_trimmed)
}

fn collect_schema_errors(value: &serde_json::Value, schema: &JsonSchema) -> Vec<String> {
    let Some(obj) = value.as_object() else {
        return vec!["Top-level JSON value must be an object".to_string()];
    };
    let mut errors: Vec<String> = schema
        .required_fields
        .iter()
        .filter(|f| !obj.contains_key(**f))
        .map(|f| format!("Missing required field '{}'", f))
        .collect();
    for (field, field_type) in &schema.field_types {
        if let Some(v) = obj.get(*field) {
            if !field_type.matches(v) {
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
    errors
}

pub(crate) fn validate_schema(
    value: &serde_json::Value,
    schema: &JsonSchema,
) -> Result<(), SchemaValidationError> {
    let errors = collect_schema_errors(value, schema);
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

fn try_extract_and_parse<T: serde::de::DeserializeOwned + 'static>(raw: &str) -> Option<Result<T>> {
    let candidates = [
        Some(raw.to_string()),
        json_parser_extract::extract_from_markdown(raw),
        json_parser_extract::extract_json_from_text(raw),
    ];
    for c in candidates.into_iter().flatten() {
        if let Ok(v) = try_parse_candidate::<T>(&c) {
            return Some(Ok(v));
        }
    }
    jsonrepair(raw)
        .ok()
        .and_then(|repaired| try_parse_candidate::<T>(&repaired).ok())
        .map(Ok)
}

pub(crate) fn parse_with_repair<T: serde::de::DeserializeOwned + 'static>(raw: &str) -> Result<T> {
    let raw_trimmed = raw.trim();
    if let Ok(value) = try_parse_candidate::<T>(raw_trimmed) {
        return Ok(value);
    }
    if let Some(result) = try_extract_and_parse::<T>(raw_trimmed) {
        return result;
    }
    if let Some(fallback) = regex_fallback_value::<T>(raw_trimmed) {
        if let Some(schema) = schema_for_type::<T>() {
            validate_schema(&fallback, &schema)?;
        }
        return Ok(serde_json::from_value(fallback)?);
    }
    anyhow::bail!("Unable to parse JSON after direct parse, extraction, repair, and fallback")
}

fn detect_parse_method(raw_trimmed: &str) -> ParseMethod {
    if raw_trimmed.starts_with("```") {
        ParseMethod::JsonMarkdown
    } else if raw_trimmed.starts_with('{') && raw_trimmed.ends_with('}') {
        ParseMethod::JsonDirect
    } else {
        ParseMethod::JsonExtracted
    }
}

fn build_parse_result(json: serde_json::Value, method: ParseMethod) -> IntelParseResult {
    IntelParseResult {
        choice: json_field(&json, "choice"),
        label: json_field(&json, "label"),
        reason: json_field(&json, "reason"),
        entropy: json_field_f64(&json, "entropy"),
        cmd: json_field(&json, "cmd"),
        parse_method: method,
    }
}

fn try_legacy_token(
    raw_trimmed: &str,
    pairs: &'static [(&'static str, &'static str)],
) -> Option<IntelParseResult> {
    let token = raw_trimmed
        .trim_matches(|c: char| c == '"' || c == '\'')
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim();
    pairs
        .iter()
        .find(|(code, label)| token == *code || token.eq_ignore_ascii_case(*label))
        .map(|(code, label)| IntelParseResult {
            choice: Some((*code).to_string()),
            label: Some((*label).to_string()),
            reason: None,
            entropy: None,
            cmd: None,
            parse_method: ParseMethod::LegacyToken,
        })
}

pub(crate) fn parse_intel_output(
    raw: &str,
    pairs: &'static [(&'static str, &'static str)],
) -> IntelParseResult {
    let raw_trimmed = raw.trim();
    if let Some(json) = extract_json_object(raw_trimmed) {
        return build_parse_result(json, detect_parse_method(raw_trimmed));
    }
    if let Some(result) = try_legacy_token(raw_trimmed, pairs) {
        return result;
    }
    IntelParseResult {
        choice: None,
        label: None,
        reason: None,
        entropy: None,
        cmd: None,
        parse_method: ParseMethod::Failed,
    }
}

pub(crate) fn validate_entropy(entropy: f64) -> f64 {
    entropy.clamp(0.0, 1.0)
}

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
        let raw = r#"{"choice": "1", "label": "CHAT",}"#;
        let result = parse_intel_output(raw, TEST_PAIRS);
        assert_eq!(result.label, Some("CHAT".to_string()));
    }

    #[test]
    fn test_repair_missing_quotes() {
        let raw = r#"{choice: "1", label: CHAT}"#;
        let result = parse_intel_output(raw, TEST_PAIRS);
        assert_eq!(result.label, Some("CHAT".to_string()));
    }

    #[test]
    fn test_repair_single_quotes() {
        let raw = r#"{'choice': '1', 'label': 'CHAT'}"#;
        let result = parse_intel_output(raw, TEST_PAIRS);
        assert_eq!(result.label, Some("CHAT".to_string()));
    }
}
