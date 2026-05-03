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
use miette::{Diagnostic, SourceSpan};
use std::any::TypeId;
use thiserror::Error;

#[derive(Error, Diagnostic, Debug)]
pub(crate) enum ParseError {
    #[error("Unable to parse JSON after direct parse, extraction, repair, and fallback")]
    #[diagnostic(
        code(elma::json::unable_to_parse),
        help("The model output did not contain valid JSON or a recognizable fallback pattern.")
    )]
    UnableToParse {
        #[source_code]
        input: String,
        #[label("this output")]
        span: SourceSpan,
    },
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Schema(#[from] SchemaValidationError),
}

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
    let mut raw_trimmed = raw.trim().to_string();

    // Strip <think>...</think> blocks that leak from models even when
    // reasoning_format=none. This prevents the parser from failing to
    // find JSON buried inside thinking output.
    loop {
        if let Some(start) = raw_trimmed.find("<think>") {
            if let Some(end) = raw_trimmed.find("</think>") {
                raw_trimmed.replace_range(start..end + "</think>".len(), "");
                continue;
            }
        }
        break;
    }
    let raw_trimmed = raw_trimmed.trim();

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

/// Quick pre-repair for common small-model JSON errors.
/// Applied before the main repair pipeline to handle frequent failure patterns.
fn pre_repair_json(raw: &str) -> String {
    let mut s = raw.to_string();

    // 1. Fix trailing commas in objects: {"a": 1,} -> {"a": 1}
    // Pattern: comma followed by optional whitespace then closing brace
    s = regex::Regex::new(r",\s*}").map(|r| r.replace_all(&s, "}")).unwrap_or(std::borrow::Cow::Borrowed(&s)).to_string();

    // 2. Fix trailing commas in arrays: [1, 2,] -> [1, 2]
    s = regex::Regex::new(r",\s*\]").map(|r| r.replace_all(&s, "]")).unwrap_or(std::borrow::Cow::Borrowed(&s)).to_string();

    // 3. Fix single quotes to double quotes (for keys and string values)
    s = regex::Regex::new(r"'([^']*)'").map(|r| r.replace_all(&s, "\"$1\"")).unwrap_or(std::borrow::Cow::Borrowed(&s)).to_string();

    // 4. Fix unquoted keys: {key: "value"} -> {"key": "value"}
    // This is tricky; use a conservative pattern for simple alphanumeric keys
    s = regex::Regex::new(r#"\{([^"'{}\[\]]*?)(\w[\w_]*)\s*:"#)
        .map(|r| r.replace_all(&s, "{$1\"$2\":"))
        .unwrap_or(std::borrow::Cow::Borrowed(&s))
        .to_string();

    // 5. Strip markdown fences and language tags
    s = regex::Regex::new(r"```(?:json)?\s*").map(|r| r.replace_all(&s, "")).unwrap_or(std::borrow::Cow::Borrowed(&s)).to_string();
    s = regex::Regex::new(r"\s*```").map(|r| r.replace_all(&s, "")).unwrap_or(std::borrow::Cow::Borrowed(&s)).to_string();

    s
}

/// Universal model JSON parser: tries direct parse, then pre-repair, then full repair pipeline.
/// Use this entry point for ALL model-generated JSON (tool arguments, intel outputs, etc.).
pub(crate) fn parse_model_json<T: serde::de::DeserializeOwned + 'static>(raw: &str) -> Result<T, ParseError> {
    let raw_trimmed = raw.trim();

    // Fast path: try direct parse first
    match serde_json::from_str::<T>(raw_trimmed) {
        Ok(v) => return Ok(v),
        Err(_) => {}
    }

    // Second path: try the existing extraction + repair pipeline
    if let Ok(v) = parse_with_repair::<T>(raw) {
        return Ok(v);
    }

    // Third path: apply pre-repair for common errors, then try extraction pipeline again
    let repaired = pre_repair_json(raw_trimmed);
    if let Ok(v) = parse_with_repair::<T>(&repaired) {
        return Ok(v);
    }

    Err(ParseError::UnableToParse {
        input: raw.to_string(),
        span: (0, raw.len()).into(),
    })
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
    Err(ParseError::UnableToParse {
        input: raw.to_string(),
        span: (0, raw.len()).into(),
    }
    .into())
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
    fn test_extract_json_from_think_block() {
        let raw = "<think>\nLet me analyze...\n</think>\n{\"choice\": \"1\", \"label\": \"CHAT\"}";
        let result = parse_intel_output(raw, TEST_PAIRS);
        assert_eq!(result.label, Some("CHAT".to_string()));
        assert_eq!(result.choice, Some("1".to_string()));
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

    // ── parse_model_json fuzzing tests ──

    #[test]
    fn test_parse_model_json_direct() {
        let result: serde_json::Value = parse_model_json(r#"{"path": "src/main.rs"}"#).unwrap();
        assert_eq!(result["path"], "src/main.rs");
    }

    #[test]
    fn test_parse_model_json_trailing_comma() {
        let result: serde_json::Value = parse_model_json(r#"{"a": 1, "b": 2,}"#).unwrap();
        assert_eq!(result["a"], 1);
        assert_eq!(result["b"], 2);
    }

    #[test]
    fn test_parse_model_json_unquoted_key() {
        let result: serde_json::Value = parse_model_json(r#"{path: "src/main.rs"}"#).unwrap();
        assert_eq!(result["path"], "src/main.rs");
    }

    #[test]
    fn test_parse_model_json_single_quotes() {
        let result: serde_json::Value = parse_model_json(r#"{'path': 'src/main.rs'}"#).unwrap();
        assert_eq!(result["path"], "src/main.rs");
    }

    #[test]
    fn test_parse_model_json_markdown_fence() {
        let result: serde_json::Value = parse_model_json("```json\n{\"path\": \"main.rs\"}\n```").unwrap();
        assert_eq!(result["path"], "main.rs");
    }

    #[test]
    fn test_parse_model_json_with_prose_prefix() {
        let result: serde_json::Value = parse_model_json("Here is the result: {\"path\": \"main.rs\", \"depth\": 2}").unwrap();
        assert_eq!(result["path"], "main.rs");
        assert_eq!(result["depth"], 2);
    }

    #[test]
    fn test_parse_model_json_think_block() {
        let result: serde_json::Value = parse_model_json("<think>Need to read this file</think>{\"path\": \"main.rs\"}").unwrap();
        assert_eq!(result["path"], "main.rs");
    }

    #[test]
    fn test_parse_model_json_extra_fields_ok() {
        let result: serde_json::Value = parse_model_json(r#"{"path": "main.rs", "extra": "ignored", "nested": {"a": 1}}"#).unwrap();
        assert_eq!(result["path"], "main.rs");
        assert_eq!(result["extra"], "ignored");
    }

    #[test]
    fn test_parse_model_json_trailing_text() {
        let result: serde_json::Value = parse_model_json(r#"{"path": "main.rs", "depth": 2} and then some"#).unwrap();
        assert_eq!(result["path"], "main.rs");
    }

    #[test]
    fn test_parse_model_json_empty_does_not_panic() {
        let result = parse_model_json::<serde_json::Value>("");
        assert!(result.is_ok(), "empty input should not panic; got {:?}", result);
    }

    #[test]
    fn test_parse_model_json_garbage_does_not_panic() {
        let result = parse_model_json::<serde_json::Value>("this is definitely not json at all whatsoever");
        assert!(result.is_ok(), "garbage input should not panic; got {:?}", result);
    }

    #[test]
    fn test_parse_model_json_truncated_repairs() {
        let result: Result<serde_json::Value, _> = parse_model_json(r#"{"path": "main.rs""#);
        assert!(result.is_ok(), "truncated JSON should still parse");
    }

    #[test]
    fn test_parse_model_json_array_trailing_comma() {
        let result: serde_json::Value = parse_model_json(r#"{"items": [1, 2, 3,]}"#).unwrap();
        assert_eq!(result["items"][0], 1);
        assert_eq!(result["items"][2], 3);
    }
}
