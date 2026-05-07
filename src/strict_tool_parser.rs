//! @efficiency-role: domain-logic
//! Strict tool argument parsing and model-facing error contracts.
//!
//! Provides robust JSON parsing of tool calls from model outputs with
//! strict validation and clear error messages designed for small models.

use crate::*;

/// A successfully parsed tool call from model output.
#[derive(Debug, Clone)]
pub(crate) struct ParsedToolCall {
    pub(crate) name: String,
    pub(crate) arguments: HashMap<String, String>,
    pub(crate) raw_json: String,
    pub(crate) id: Option<String>,
}

/// Classification of what went wrong during tool call parsing.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ToolParseErrorType {
    InvalidJson,
    MissingField(String),
    ExtraField(String),
    WrongType {
        field: String,
        expected: &'static str,
    },
    EmptyToolName,
    UnknownTool(String),
    MalformedInput,
}

/// A structured error produced when a tool call cannot be parsed.
#[derive(Debug, Clone)]
pub(crate) struct ToolParseError {
    pub(crate) error_type: ToolParseErrorType,
    pub(crate) message: String,
    pub(crate) raw_input: String,
    pub(crate) position: Option<usize>,
}

/// Strict and lenient tool call parser for model-generated JSON.
pub(crate) struct StrictToolParser;

impl StrictToolParser {
    /// Strict JSON parsing — input MUST be a valid JSON object
    /// with `name` and `arguments` (or `input`).
    pub(crate) fn parse(
        json: &str,
        known_tools: &[&str],
    ) -> Result<ParsedToolCall, ToolParseError> {
        let trimmed = json.trim();
        let raw_json = trimmed.to_string();

        let value: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                return Err(ToolParseError {
                    error_type: ToolParseErrorType::InvalidJson,
                    message: format!("Invalid JSON: {}", e),
                    raw_input: json.to_string(),
                    position: e.column().checked_sub(1),
                });
            }
        };

        let obj = match value.as_object() {
            Some(o) => o,
            None => {
                return Err(ToolParseError {
                    error_type: ToolParseErrorType::InvalidJson,
                    message: "Expected a JSON object at the top level".to_string(),
                    raw_input: json.to_string(),
                    position: None,
                });
            }
        };

        let name = match obj.get("name") {
            Some(v) => match v.as_str() {
                Some(s) => {
                    if s.is_empty() {
                        return Err(ToolParseError {
                            error_type: ToolParseErrorType::EmptyToolName,
                            message: "Tool name must not be empty".to_string(),
                            raw_input: json.to_string(),
                            position: None,
                        });
                    }
                    s.to_string()
                }
                None => {
                    return Err(ToolParseError {
                        error_type: ToolParseErrorType::WrongType {
                            field: "name".to_string(),
                            expected: "string",
                        },
                        message: format!("Field 'name' must be a string, got {}", v),
                        raw_input: json.to_string(),
                        position: None,
                    });
                }
            },
            None => {
                return Err(ToolParseError {
                    error_type: ToolParseErrorType::MissingField("name".to_string()),
                    message: "Missing required field: 'name'".to_string(),
                    raw_input: json.to_string(),
                    position: None,
                });
            }
        };

        if !known_tools.contains(&name.as_str()) {
            return Err(ToolParseError {
                error_type: ToolParseErrorType::UnknownTool(name.clone()),
                message: format!(
                    "Unknown tool: '{}'. Known tools: {}",
                    name,
                    known_tools.join(", ")
                ),
                raw_input: json.to_string(),
                position: None,
            });
        }

        let args_obj = match obj.get("arguments").or_else(|| obj.get("input")) {
            Some(v) => match v.as_object() {
                Some(o) => o,
                None => {
                    return Err(ToolParseError {
                        error_type: ToolParseErrorType::WrongType {
                            field: "arguments".to_string(),
                            expected: "object",
                        },
                        message: format!("Field 'arguments' must be a JSON object, got {}", v),
                        raw_input: json.to_string(),
                        position: None,
                    });
                }
            },
            None => {
                return Err(ToolParseError {
                    error_type: ToolParseErrorType::MissingField("arguments".to_string()),
                    message: "Missing required field: 'arguments' (or 'input')".to_string(),
                    raw_input: json.to_string(),
                    position: None,
                });
            }
        };

        let arguments: HashMap<String, String> = args_obj
            .iter()
            .map(|(k, v)| (k.clone(), value_to_string(v)))
            .collect();

        let allowed: &[&str] = &["name", "arguments", "input", "id"];
        for key in obj.keys() {
            if !allowed.contains(&key.as_str()) {
                return Err(ToolParseError {
                    error_type: ToolParseErrorType::ExtraField(key.clone()),
                    message: format!("Unexpected field: '{}'. Allowed fields: {:?}", key, allowed),
                    raw_input: json.to_string(),
                    position: None,
                });
            }
        }

        let id = obj.get("id").and_then(|v| v.as_str()).map(String::from);

        Ok(ParsedToolCall {
            name,
            arguments,
            raw_json,
            id,
        })
    }

    /// Lenient parsing — extracts tool calls from surrounding text, markdown fences, etc.
    pub(crate) fn parse_lenient(
        text: &str,
        known_tools: &[&str],
    ) -> Vec<Result<ParsedToolCall, ToolParseError>> {
        let mut results: Vec<Result<ParsedToolCall, ToolParseError>> = Vec::new();
        let text = text.trim();

        match Self::parse(text, known_tools) {
            Ok(call) => {
                results.push(Ok(call));
                return results;
            }
            Err(_) => {}
        }

        let mut saw_code_block = false;
        for block in extract_json_code_blocks(text) {
            saw_code_block = true;
            match Self::parse(&block, known_tools) {
                Ok(call) => results.push(Ok(call)),
                Err(e) => results.push(Err(e)),
            }
        }

        if !saw_code_block {
            if let Some(json_str) = find_json_object(text) {
                match Self::parse(&json_str, known_tools) {
                    Ok(call) => results.push(Ok(call)),
                    Err(e) => results.push(Err(e)),
                }
            } else {
                results.push(Err(ToolParseError {
                    error_type: ToolParseErrorType::MalformedInput,
                    message: "Could not find a valid JSON tool call in the input".to_string(),
                    raw_input: text.to_string(),
                    position: None,
                }));
            }
        }

        results
    }

    /// Format an error into a model-friendly message.
    pub(crate) fn format_error(error: &ToolParseError, _model_name: &str) -> String {
        let tool_name = match &error.error_type {
            ToolParseErrorType::UnknownTool(name) => name.clone(),
            _ => String::new(),
        };
        ModelFacingErrorContract::render(
            &error.message,
            &tool_name,
            ModelFacingErrorContract::retry_suggestion(error).as_deref(),
        )
    }

    /// Whether retrying the same input might succeed (recoverable errors).
    pub(crate) fn is_recoverable(error: &ToolParseError) -> bool {
        matches!(
            error.error_type,
            ToolParseErrorType::InvalidJson
                | ToolParseErrorType::MalformedInput
                | ToolParseErrorType::MissingField(_)
                | ToolParseErrorType::WrongType { .. }
        )
    }
}

/// Standardized error contract for model-facing tool call errors.
pub(crate) struct ModelFacingErrorContract;

impl ModelFacingErrorContract {
    /// Render a standardized error message for the model.
    pub(crate) fn render(message: &str, tool_name: &str, suggestion: Option<&str>) -> String {
        let mut output = format!("[ToolParseError] {}", message);
        if !tool_name.is_empty() {
            output.push_str(&format!("\nTool: {}", tool_name));
        }
        if let Some(s) = suggestion {
            output.push_str(&format!("\nSuggestion: {}", s));
        }
        output
    }

    /// Suggest how the model can fix a particular parse error.
    pub(crate) fn retry_suggestion(error: &ToolParseError) -> Option<String> {
        match &error.error_type {
            ToolParseErrorType::InvalidJson => Some(
                "Ensure the tool call is valid JSON: use double quotes, \
                 proper commas, and no trailing commas."
                    .to_string(),
            ),
            ToolParseErrorType::MissingField(field) => Some(format!(
                "Add the missing '{}' field to your tool call.",
                field
            )),
            ToolParseErrorType::ExtraField(field) => Some(format!(
                "Remove the unexpected '{}' field. \
                 Only 'name', 'arguments', 'input', and 'id' are allowed.",
                field
            )),
            ToolParseErrorType::WrongType { field, expected } => Some(format!(
                "The '{}' field must be of type {}.",
                field, expected
            )),
            ToolParseErrorType::EmptyToolName => Some("Provide a non-empty tool name.".to_string()),
            ToolParseErrorType::UnknownTool(name) => Some(format!(
                "Use one of the available tools instead of '{}'.",
                name
            )),
            ToolParseErrorType::MalformedInput => Some(
                "Wrap your tool call in a JSON code block:\n\
                 ```json\n\
                 {\"name\": \"tool_name\", \"arguments\": {{...}}}\n\
                 ```"
                .to_string(),
            ),
        }
    }
}

// ── Helper functions ──

/// Convert a serde_json::Value to its string representation.
fn value_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// Extract JSON objects from markdown code blocks (```json ... ```).
fn extract_json_code_blocks(text: &str) -> Vec<String> {
    let mut blocks: Vec<String> = Vec::new();
    let mut in_block = false;
    let mut current = String::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            if in_block {
                if !current.trim().is_empty() {
                    blocks.push(current.trim().to_string());
                }
                current.clear();
                in_block = false;
            } else {
                in_block = true;
                current.clear();
            }
            continue;
        }
        if in_block {
            current.push_str(line);
            current.push('\n');
        }
    }

    if in_block && !current.trim().is_empty() {
        blocks.push(current.trim().to_string());
    }

    blocks
}

/// Try to find the first balanced JSON object `{...}` in arbitrary text.
fn find_json_object(text: &str) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut start: Option<usize> = None;
    let mut depth: i32 = 0;
    let mut in_string = false;
    let mut escaped = false;

    for (i, &c) in chars.iter().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        if c == '\\' && in_string {
            escaped = true;
            continue;
        }
        if c == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }

        if c == '{' {
            if start.is_none() {
                start = Some(i);
            }
            depth += 1;
        } else if c == '}' {
            depth -= 1;
            if depth == 0 {
                if let Some(s) = start {
                    let json_str: String = chars[s..=i].iter().collect();
                    return Some(json_str);
                }
            }
        }
    }

    None
}

impl ToolParseError {
    /// Whether this error type is recoverable by retrying with different input.
    pub(crate) fn is_recoverable(&self) -> bool {
        StrictToolParser::is_recoverable(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Valid calls ──

    #[test]
    fn test_parse_valid_call_with_arguments() {
        let json = r#"{"name": "bash", "arguments": {"command": "ls -la"}}"#;
        let result = StrictToolParser::parse(json, &["bash", "read"]).unwrap();
        assert_eq!(result.name, "bash");
        assert_eq!(
            result.arguments.get("command").map(|s| s.as_str()),
            Some("ls -la")
        );
        assert!(result.id.is_none());
    }

    #[test]
    fn test_parse_valid_call_with_input() {
        let json = r#"{"name": "read", "input": {"filePath": "src/main.rs"}}"#;
        let result = StrictToolParser::parse(json, &["bash", "read"]).unwrap();
        assert_eq!(result.name, "read");
        assert_eq!(
            result.arguments.get("filePath").map(|s| s.as_str()),
            Some("src/main.rs")
        );
    }

    #[test]
    fn test_parse_valid_call_with_id() {
        let json = r#"{"name": "bash", "arguments": {"command": "echo hi"}, "id": "call_123"}"#;
        let result = StrictToolParser::parse(json, &["bash"]).unwrap();
        assert_eq!(result.id, Some("call_123".to_string()));
    }

    #[test]
    fn test_parse_valid_call_whitespace() {
        let json = "  {\"name\": \"bash\", \"arguments\": {\"command\": \"ls\"}}  ";
        let result = StrictToolParser::parse(json, &["bash"]).unwrap();
        assert_eq!(result.name, "bash");
    }

    // ── Error: InvalidJson ──

    #[test]
    fn test_parse_invalid_json() {
        let json = r#"{name: bash}"#;
        let err = StrictToolParser::parse(json, &["bash"]).unwrap_err();
        assert_eq!(err.error_type, ToolParseErrorType::InvalidJson);
        assert!(err.message.contains("Invalid JSON"));
    }

    #[test]
    fn test_parse_empty_input() {
        let err = StrictToolParser::parse("", &["bash"]).unwrap_err();
        assert_eq!(err.error_type, ToolParseErrorType::InvalidJson);
    }

    #[test]
    fn test_parse_not_an_object() {
        let json = r#""just a string""#;
        let err = StrictToolParser::parse(json, &["bash"]).unwrap_err();
        assert_eq!(err.error_type, ToolParseErrorType::InvalidJson);
    }

    // ── Error: MissingField ──

    #[test]
    fn test_parse_missing_name() {
        let json = r#"{"arguments": {"cmd": "ls"}}"#;
        let err = StrictToolParser::parse(json, &["bash"]).unwrap_err();
        assert_eq!(
            err.error_type,
            ToolParseErrorType::MissingField("name".to_string())
        );
    }

    #[test]
    fn test_parse_missing_arguments() {
        let json = r#"{"name": "bash"}"#;
        let err = StrictToolParser::parse(json, &["bash"]).unwrap_err();
        assert_eq!(
            err.error_type,
            ToolParseErrorType::MissingField("arguments".to_string())
        );
    }

    // ── Error: ExtraField ──

    #[test]
    fn test_parse_extra_field() {
        let json = r#"{"name": "bash", "arguments": {}, "extra": "bad"}"#;
        let err = StrictToolParser::parse(json, &["bash"]).unwrap_err();
        assert_eq!(
            err.error_type,
            ToolParseErrorType::ExtraField("extra".to_string())
        );
    }

    // ── Error: WrongType ──

    #[test]
    fn test_parse_wrong_type_name() {
        let json = r#"{"name": 42, "arguments": {}}"#;
        let err = StrictToolParser::parse(json, &["bash"]).unwrap_err();
        assert_eq!(
            err.error_type,
            ToolParseErrorType::WrongType {
                field: "name".to_string(),
                expected: "string"
            }
        );
    }

    #[test]
    fn test_parse_wrong_type_arguments() {
        let json = r#"{"name": "bash", "arguments": "not_an_object"}"#;
        let err = StrictToolParser::parse(json, &["bash"]).unwrap_err();
        assert_eq!(
            err.error_type,
            ToolParseErrorType::WrongType {
                field: "arguments".to_string(),
                expected: "object"
            }
        );
    }

    // ── Error: EmptyToolName ──

    #[test]
    fn test_parse_empty_tool_name() {
        let json = r#"{"name": "", "arguments": {}}"#;
        let err = StrictToolParser::parse(json, &["bash"]).unwrap_err();
        assert_eq!(err.error_type, ToolParseErrorType::EmptyToolName);
    }

    // ── Error: UnknownTool ──

    #[test]
    fn test_parse_unknown_tool() {
        let json = r#"{"name": "unknown_tool", "arguments": {}}"#;
        let err = StrictToolParser::parse(json, &["bash", "read"]).unwrap_err();
        assert!(matches!(err.error_type, ToolParseErrorType::UnknownTool(_)));
    }

    // ── Lenient mode ──

    #[test]
    fn test_parse_lenient_markdown_code_block() {
        let text =
            "Here's the result:\n```json\n{\"name\": \"bash\", \"arguments\": {\"command\": \"ls\"}}\n```";
        let results = StrictToolParser::parse_lenient(text, &["bash", "read"]);
        assert_eq!(results.len(), 1);
        let call = results[0].as_ref().unwrap();
        assert_eq!(call.name, "bash");
    }

    #[test]
    fn test_parse_lenient_surrounding_text() {
        let text = "I should call: {\"name\": \"read\", \"arguments\": {\"filePath\": \"Cargo.toml\"}} and then check the output.";
        let results = StrictToolParser::parse_lenient(text, &["read"]);
        assert_eq!(results.len(), 1);
        let call = results[0].as_ref().unwrap();
        assert_eq!(call.name, "read");
    }

    #[test]
    fn test_parse_lenient_no_tool_call() {
        let text = "This is just a normal response with no tool call.";
        let results = StrictToolParser::parse_lenient(text, &["bash"]);
        assert!(!results.is_empty());
        assert!(results[0].is_err());
    }

    #[test]
    fn test_parse_lenient_direct_preferred() {
        let text = r#"{"name": "bash", "arguments": {"command": "ls"}}"#;
        let results = StrictToolParser::parse_lenient(text, &["bash"]);
        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());
    }

    // ── Error formatting ──

    #[test]
    fn test_format_error_missing_field() {
        let err = ToolParseError {
            error_type: ToolParseErrorType::MissingField("name".to_string()),
            message: "Missing required field: 'name'".to_string(),
            raw_input: r#"{"args": {}}"#.to_string(),
            position: None,
        };
        let formatted = StrictToolParser::format_error(&err, "test-model");
        assert!(formatted.contains("Missing required field"));
        assert!(formatted.contains("Suggestion:"));
        assert!(formatted.contains("'name'"));
    }

    #[test]
    fn test_format_error_unknown_tool() {
        let err = ToolParseError {
            error_type: ToolParseErrorType::UnknownTool("bad_tool".to_string()),
            message: "Unknown tool: 'bad_tool'".to_string(),
            raw_input: r#"{"name": "bad_tool", "arguments": {}}"#.to_string(),
            position: None,
        };
        let formatted = StrictToolParser::format_error(&err, "test-model");
        assert!(formatted.contains("bad_tool"));
    }

    #[test]
    fn test_format_error_invalid_json() {
        let err = ToolParseError {
            error_type: ToolParseErrorType::InvalidJson,
            message: "Invalid JSON: expected value at line 1 column 1".to_string(),
            raw_input: "{invalid".to_string(),
            position: Some(0),
        };
        let formatted = StrictToolParser::format_error(&err, "test-model");
        assert!(formatted.contains("Invalid JSON"));
        assert!(formatted.contains("Suggestion:"));
    }

    // ── is_recoverable ──

    #[test]
    fn test_is_recoverable_true_for_invalid_json() {
        let err = ToolParseError {
            error_type: ToolParseErrorType::InvalidJson,
            message: String::new(),
            raw_input: String::new(),
            position: None,
        };
        assert!(StrictToolParser::is_recoverable(&err));
    }

    #[test]
    fn test_is_recoverable_true_for_missing_field() {
        let err = ToolParseError {
            error_type: ToolParseErrorType::MissingField("name".to_string()),
            message: String::new(),
            raw_input: String::new(),
            position: None,
        };
        assert!(StrictToolParser::is_recoverable(&err));
    }

    #[test]
    fn test_is_recoverable_true_for_wrong_type() {
        let err = ToolParseError {
            error_type: ToolParseErrorType::WrongType {
                field: "name".to_string(),
                expected: "string",
            },
            message: String::new(),
            raw_input: String::new(),
            position: None,
        };
        assert!(StrictToolParser::is_recoverable(&err));
    }

    #[test]
    fn test_is_recoverable_false_for_empty_name() {
        let err = ToolParseError {
            error_type: ToolParseErrorType::EmptyToolName,
            message: String::new(),
            raw_input: String::new(),
            position: None,
        };
        assert!(!StrictToolParser::is_recoverable(&err));
    }

    #[test]
    fn test_is_recoverable_false_for_unknown_tool() {
        let err = ToolParseError {
            error_type: ToolParseErrorType::UnknownTool("nope".to_string()),
            message: String::new(),
            raw_input: String::new(),
            position: None,
        };
        assert!(!StrictToolParser::is_recoverable(&err));
    }

    #[test]
    fn test_is_recoverable_false_for_extra_field() {
        let err = ToolParseError {
            error_type: ToolParseErrorType::ExtraField("extra".to_string()),
            message: String::new(),
            raw_input: String::new(),
            position: None,
        };
        assert!(!StrictToolParser::is_recoverable(&err));
    }

    // ── ModelFacingErrorContract ──

    #[test]
    fn test_contract_render_basic() {
        let msg = ModelFacingErrorContract::render("Test error", "bash", Some("Fix it"));
        assert!(msg.contains("Test error"));
        assert!(msg.contains("bash"));
        assert!(msg.contains("Fix it"));
    }

    #[test]
    fn test_contract_render_no_suggestion() {
        let msg = ModelFacingErrorContract::render("Simple error", "", None);
        assert!(msg.contains("Simple error"));
        assert!(!msg.contains("Suggestion:"));
    }

    #[test]
    fn test_contract_retry_suggestion_all_types() {
        let variants: Vec<(ToolParseErrorType, &str)> = vec![
            (ToolParseErrorType::InvalidJson, "Ensure"),
            (ToolParseErrorType::MissingField("name".to_string()), "Add"),
            (ToolParseErrorType::ExtraField("bad".to_string()), "Remove"),
            (
                ToolParseErrorType::WrongType {
                    field: "name".to_string(),
                    expected: "string",
                },
                "must be",
            ),
            (ToolParseErrorType::EmptyToolName, "non-empty"),
            (ToolParseErrorType::UnknownTool("x".to_string()), "instead"),
            (ToolParseErrorType::MalformedInput, "code block"),
        ];
        for (typ, keyword) in variants {
            let err = ToolParseError {
                error_type: typ,
                message: String::new(),
                raw_input: String::new(),
                position: None,
            };
            let suggestion = ModelFacingErrorContract::retry_suggestion(&err);
            assert!(
                suggestion.is_some(),
                "Expected suggestion for {:?}",
                err.error_type
            );
            assert!(
                suggestion.unwrap().contains(keyword),
                "Expected '{}' in suggestion for {:?}",
                keyword,
                err.error_type
            );
        }
    }

    // ── Helper: extract_json_code_blocks ──

    #[test]
    fn test_extract_code_blocks_simple() {
        let text = "```json\n{\"a\": 1}\n```";
        let blocks = extract_json_code_blocks(text);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0], "{\"a\": 1}");
    }

    #[test]
    fn test_extract_code_blocks_multiple() {
        let text = "```json\n{\"a\": 1}\n```\nSome text\n```\n{\"b\": 2}\n```";
        let blocks = extract_json_code_blocks(text);
        assert_eq!(blocks.len(), 2);
    }

    #[test]
    fn test_extract_code_blocks_no_fence() {
        let blocks = extract_json_code_blocks("just text");
        assert!(blocks.is_empty());
    }

    #[test]
    fn test_extract_code_blocks_unclosed() {
        let text = "```json\n{\"a\": 1}\n";
        let blocks = extract_json_code_blocks(text);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0], "{\"a\": 1}");
    }

    // ── Helper: find_json_object ──

    #[test]
    fn test_find_json_object_simple() {
        let text = "prefix {\"a\": 1} suffix";
        let result = find_json_object(text);
        assert_eq!(result, Some("{\"a\": 1}".to_string()));
    }

    #[test]
    fn test_find_json_object_nested() {
        let text = r#"prefix {"a": {"b": 2}} suffix"#;
        let result = find_json_object(text);
        assert_eq!(result, Some(r#"{"a": {"b": 2}}"#.to_string()));
    }

    #[test]
    fn test_find_json_object_no_object() {
        let result = find_json_object("no braces here");
        assert!(result.is_none());
    }

    #[test]
    fn test_find_json_object_string_with_braces() {
        let text = r#"prefix {"msg": "hello {world}"} suffix"#;
        let result = find_json_object(text);
        assert!(result.is_some());
        let json = result.unwrap();
        assert!(json.contains("hello {world}"));
    }

    #[test]
    fn test_find_json_object_multiple_objects() {
        let text = r#"first {"a": 1} second {"b": 2}"#;
        let result = find_json_object(text);
        assert_eq!(result, Some("{\"a\": 1}".to_string()));
    }

    // ── Convenience method on ToolParseError ──

    #[test]
    fn test_tool_parse_error_is_recoverable() {
        let err = ToolParseError {
            error_type: ToolParseErrorType::InvalidJson,
            message: String::new(),
            raw_input: String::new(),
            position: None,
        };
        assert!(err.is_recoverable());

        let err = ToolParseError {
            error_type: ToolParseErrorType::EmptyToolName,
            message: String::new(),
            raw_input: String::new(),
            position: None,
        };
        assert!(!err.is_recoverable());
    }
}
