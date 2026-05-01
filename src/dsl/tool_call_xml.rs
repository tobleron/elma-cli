//! Parser for provider-style tool markup.
//!
//! Some fine-tuned models produce native `<tool_call>` XML instead of Elma's
//! compact action DSL. This module provides a fallback parser that converts
//! exact provider-style markup into `AgentAction`, so those models work without
//! entering the DSL repair loop. This is a boundary adapter, not a general XML
//! extractor.

use crate::dsl::action::AgentAction;
use crate::dsl::error::ParseContext;
use crate::types_api::ToolCall;
use serde_json::Value;

/// Try to parse `<tool_call>JSON</tool_call>` XML format as an `AgentAction`.
///
/// Handles optional Markdown fences around the `<tool_call>` tags.
/// Returns `None` if no valid `<tool_call>` is found.
pub fn parse_tool_call_xml(text: &str) -> Option<AgentAction> {
    let json_str = exact_tag_body(text, "tool_call")?;
    parse_tool_call_json(&json_str)
}

/// Try to parse exact `<command>...</command>` or
/// `<execute_command><command>...</command></execute_command>` markup.
///
/// The body is first parsed as native action DSL. If that fails, the body is
/// treated as a shell command and mapped to `X`.
pub fn parse_command_xml(text: &str) -> Option<AgentAction> {
    let body = if let Some(body) = exact_tag_body(text, "command") {
        body
    } else {
        let outer = exact_tag_body(text, "execute_command")?;
        exact_tag_body(&outer, "command")?
    };

    let command = body.trim();
    if command.is_empty() {
        return None;
    }

    let ctx = ParseContext {
        dsl_variant: "action",
        line: None,
    };
    if let Ok(action) = crate::dsl::action::parse_action_dsl(command, &ctx) {
        return Some(action);
    }

    Some(AgentAction::RunCommand {
        command: command.to_string(),
    })
}

fn exact_tag_body(text: &str, tag: &str) -> Option<String> {
    let text = strip_fences(text);
    let text = text.trim();
    let start_tag = format!("<{tag}>");
    let end_tag = format!("</{tag}>");
    let body = text.strip_prefix(&start_tag)?.strip_suffix(&end_tag)?;
    if body.contains(&start_tag) || body.contains(&end_tag) {
        return None;
    }
    Some(body.trim().to_string())
}

/// Try to parse bare JSON tool call format as an `AgentAction`.
///
/// Handles both `{"name":"...","arguments":{...}}` (OpenAI-style) and
/// `{"action":"...","params":{...}}` (alternate) formats.
/// This is useful for runtimes that return tool-call-shaped JSON in the
/// assistant content field instead of provider-native `tool_calls`.
pub fn parse_tool_call_json(text: &str) -> Option<AgentAction> {
    let text = strip_fences(text);
    let text = text.trim();

    let value: Value = serde_json::from_str(text).ok()?;

    // Try OpenAI-style: {"name":"...","arguments":{...}}
    if let Some(name) = value.get("name").and_then(|v| v.as_str()) {
        if value.get("arguments").and_then(|v| v.as_object()).is_some() {
            if let Some(arguments) = value.get("arguments") {
                return convert_to_action(name, arguments);
            }
        }
    }

    // Try alternate: {"action":"...","params":{...}}
    if let Some(name) = value.get("action").and_then(|v| v.as_str()) {
        let arguments = value.get("params").or_else(|| value.get("parameters"))?;
        return convert_to_action(name, arguments);
    }

    // Try flat: {"name":"read","path":"src/main.rs"}
    // where arguments are merged into the top-level object
    if let Some(name) = value.get("name").and_then(|v| v.as_str()) {
        return convert_to_action(name, &value);
    }

    None
}

/// Remove Markdown fences (``` or ~~~) if present around the content.
fn strip_fences(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
        // Find the first newline after the opening fence
        if let Some(nl) = trimmed.find('\n') {
            let after_fence = &trimmed[nl + 1..];
            // Strip trailing fence if present
            let content = if let Some(end_fence) = after_fence.rfind("```") {
                &after_fence[..end_fence]
            } else if let Some(end_fence) = after_fence.rfind("~~~") {
                &after_fence[..end_fence]
            } else {
                after_fence
            };
            return content.trim().to_string();
        }
    }
    trimmed.to_string()
}

/// Try to extract a string value from JSON arguments, trying multiple key names.
fn get_str<'a>(args: &'a Value, keys: &[&str]) -> Option<&'a str> {
    for key in keys {
        if let Some(v) = args.get(*key).and_then(|v| v.as_str()) {
            if !v.is_empty() {
                return Some(v);
            }
        }
    }
    None
}

/// Try to extract a u8 value from JSON arguments, trying multiple key names,
/// falling back to a default.
fn get_u8(args: &Value, keys: &[&str], default: u8) -> u8 {
    for key in keys {
        if let Some(v) = args.get(*key).and_then(|v| v.as_u64()) {
            return v.min(u64::from(u8::MAX)) as u8;
        }
    }
    default
}

fn convert_to_action(name: &str, args: &Value) -> Option<AgentAction> {
    match name {
        "read" | "view" => {
            let path = get_str(args, &["path", "file_path", "file"])?;
            Some(AgentAction::ReadFile {
                path: path.to_string(),
            })
        }
        "ls" | "list" => {
            let path = get_str(args, &["path", "file_path", "directory", "dir"])
                .unwrap_or(".")
                .to_string();
            let depth = get_u8(args, &["depth", "max_depth", "levels"], 1);
            Some(AgentAction::ListFiles { path, depth })
        }
        "search" | "grep" | "text_search" | "find" => {
            let q = get_str(args, &["q", "pattern", "query", "text", "search_term"])?;
            let path = get_str(args, &["path", "file_path", "directory", "dir"])
                .unwrap_or(".")
                .to_string();
            Some(AgentAction::SearchText {
                q: q.to_string(),
                path,
            })
        }
        "search_symbol" | "symbol_search" | "find_references" | "search_references" => {
            let q = get_str(args, &["q", "symbol", "name", "query", "identifier"])?;
            let path = get_str(args, &["path", "file_path", "directory", "dir"])
                .unwrap_or(".")
                .to_string();
            Some(AgentAction::SearchSymbol {
                q: q.to_string(),
                path,
            })
        }
        "edit" => {
            let path = get_str(args, &["path", "file_path", "file"])?;
            let old = get_str(args, &["old", "old_string", "search", "old_text", "find"])
                .unwrap_or_default()
                .to_string();
            let new = get_str(
                args,
                &["new", "new_string", "replace", "new_text", "replace_with"],
            )
            .unwrap_or_default()
            .to_string();
            Some(AgentAction::EditFile {
                path: path.to_string(),
                old,
                new,
            })
        }
        "shell" | "bash" | "command" | "execute_command" | "run" | "execute" => {
            let command = get_str(args, &["command", "cmd", "shell_command", "code", "script"])?;
            Some(AgentAction::RunCommand {
                command: command.to_string(),
            })
        }
        "ask" | "question" | "clarify" => {
            let question = get_str(args, &["question", "text", "message", "content"])?;
            Some(AgentAction::Ask {
                question: question.to_string(),
            })
        }
        "respond" | "done" | "answer" | "completion" | "attempt_completion" | "finalize"
        | "finish" | "reply" => {
            let summary = get_str(
                args,
                &["text", "summary", "result", "content", "message", "output"],
            )
            .unwrap_or("")
            .to_string();
            Some(AgentAction::Done { summary })
        }
        _ => None,
    }
}

/// Convert a provider-native `ToolCall` into an `AgentAction`.
///
/// Some providers return structured `tool_calls` even when `tools: None`
/// was requested. This adapter converts those tool calls into the same
/// `AgentAction` execution path used by text DSL, so the tool loop can
/// process them uniformly.
pub fn convert_tool_call_to_action(tc: &ToolCall) -> Option<AgentAction> {
    let args: Value = serde_json::from_str(&tc.function.arguments).ok()?;
    convert_to_action(&tc.function.name, &args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_read_tool_call() {
        let result = parse_tool_call_xml(
            r#"<tool_call>{"name":"read","arguments":{"path":"src/main.rs"}}</tool_call>"#,
        )
        .unwrap();
        assert!(matches!(result, AgentAction::ReadFile { path } if path == "src/main.rs"));
    }

    #[test]
    fn parse_shell_tool_call() {
        let result = parse_tool_call_xml(
            r#"<tool_call>{"name":"shell","arguments":{"command":"cargo test"}}</tool_call>"#,
        )
        .unwrap();
        assert!(matches!(result, AgentAction::RunCommand { command } if command == "cargo test"));
    }

    #[test]
    fn parse_list_tool_call() {
        let result = parse_tool_call_xml(
            r#"<tool_call>{"name":"list","arguments":{"path":"src","depth":2}}</tool_call>"#,
        )
        .unwrap();
        assert!(
            matches!(result, AgentAction::ListFiles { path, depth } if path == "src" && depth == 2)
        );
    }

    #[test]
    fn parse_search_tool_call() {
        let result = parse_tool_call_xml(
            r#"<tool_call>{"name":"search","arguments":{"pattern":"fn main","path":"src"}}</tool_call>"#,
        )
        .unwrap();
        assert!(
            matches!(result, AgentAction::SearchText { q, path } if q == "fn main" && path == "src")
        );
    }

    #[test]
    fn parse_search_symbol_tool_call() {
        let result = parse_tool_call_xml(
            r#"<tool_call>{"name":"search_symbol","arguments":{"symbol":"parse_action","path":"src"}}</tool_call>"#,
        )
        .unwrap();
        assert!(
            matches!(result, AgentAction::SearchSymbol { q, path } if q == "parse_action" && path == "src")
        );
    }

    #[test]
    fn parse_edit_tool_call() {
        let result = parse_tool_call_xml(
            r#"<tool_call>{"name":"edit","arguments":{"path":"src/main.rs","old":"foo","new":"bar"}}</tool_call>"#,
        )
        .unwrap();
        assert!(
            matches!(result, AgentAction::EditFile { path, old, new } if path == "src/main.rs" && old == "foo" && new == "bar")
        );
    }

    #[test]
    fn parse_respond_tool_call() {
        let result = parse_tool_call_xml(
            r#"<tool_call>{"name":"respond","arguments":{"text":"Task complete."}}</tool_call>"#,
        )
        .unwrap();
        assert!(matches!(result, AgentAction::Done { summary } if summary == "Task complete."));
    }

    #[test]
    fn parse_ask_tool_call() {
        let result = parse_tool_call_xml(
            r#"<tool_call>{"name":"ask","arguments":{"question":"What is your goal?"}}</tool_call>"#,
        )
        .unwrap();
        assert!(
            matches!(result, AgentAction::Ask { question } if question == "What is your goal?")
        );
    }

    #[test]
    fn parse_with_fences() {
        let input = "```\n<tool_call>{\"name\":\"read\",\"arguments\":{\"path\":\"src/main.rs\"}}</tool_call>\n```";
        let result = parse_tool_call_xml(input).unwrap();
        assert!(matches!(result, AgentAction::ReadFile { path } if path == "src/main.rs"));
    }

    #[test]
    fn reject_fences_with_text_before() {
        let input = "Here's what I'll do:\n```\n<tool_call>{\"name\":\"shell\",\"arguments\":{\"command\":\"ls\"}}</tool_call>\n```";
        assert!(parse_tool_call_xml(input).is_none());
    }

    #[test]
    fn reject_text_before_and_after() {
        let input = "I'll read the file\n<tool_call>{\"name\":\"read\",\"arguments\":{\"path\":\"Cargo.toml\"}}</tool_call>\nLet me check.";
        assert!(parse_tool_call_xml(input).is_none());
    }

    #[test]
    fn reject_empty_input() {
        assert!(parse_tool_call_xml("").is_none());
    }

    #[test]
    fn reject_no_tool_call_tags() {
        assert!(parse_tool_call_xml("just some text").is_none());
    }

    #[test]
    fn reject_malformed_json() {
        assert!(parse_tool_call_xml(
            r#"<tool_call>{"name":"read","arguments":{"path":missing}}</tool_call>"#
        )
        .is_none());
    }

    #[test]
    fn reject_unknown_tool() {
        assert!(parse_tool_call_xml(
            r#"<tool_call>{"name":"nonexistent_tool","arguments":{}}</tool_call>"#
        )
        .is_none());
    }

    #[test]
    fn reject_multiple_tool_call_blocks() {
        let input = r#"<tool_call>{"name":"read","arguments":{"path":"a.txt"}}</tool_call>
<tool_call>{"name":"read","arguments":{"path":"b.txt"}}</tool_call>"#;
        assert!(parse_tool_call_xml(input).is_none());
    }

    #[test]
    fn parse_done_via_attempt_completion() {
        let result = parse_tool_call_xml(
            r#"<tool_call>{"name":"attempt_completion","arguments":{"result":"Done!"}}</tool_call>"#,
        )
        .unwrap();
        assert!(matches!(result, AgentAction::Done { summary } if summary == "Done!"));
    }

    #[test]
    fn parse_shell_with_cmd_key() {
        let result = parse_tool_call_xml(
            r#"<tool_call>{"name":"execute_command","arguments":{"cmd":"cargo build"}}</tool_call>"#,
        )
        .unwrap();
        assert!(matches!(result, AgentAction::RunCommand { command } if command == "cargo build"));
    }

    #[test]
    fn parse_command_tag_as_shell_action() {
        let result = parse_command_xml("<command>ls -la AGENTS.md</command>").unwrap();
        assert!(
            matches!(result, AgentAction::RunCommand { command } if command == "ls -la AGENTS.md")
        );
    }

    #[test]
    fn parse_multiline_command_tag_as_shell_action() {
        let result = parse_command_xml("<command>\ncargo test dsl\n</command>").unwrap();
        assert!(
            matches!(result, AgentAction::RunCommand { command } if command == "cargo test dsl")
        );
    }

    #[test]
    fn parse_command_tag_wrapping_native_action() {
        let result = parse_command_xml("<command>\nR path=\"AGENTS.md\"\n</command>").unwrap();
        assert!(matches!(result, AgentAction::ReadFile { path } if path == "AGENTS.md"));
    }

    #[test]
    fn parse_execute_command_wrapper() {
        let result =
            parse_command_xml("<execute_command><command>ls</command></execute_command>").unwrap();
        assert!(matches!(result, AgentAction::RunCommand { command } if command == "ls"));
    }

    #[test]
    fn convert_provider_tool_call_to_read_action() {
        let tc = crate::types_api::ToolCall {
            id: "call_1".into(),
            call_type: "function".into(),
            function: crate::types_api::ToolFunctionCall {
                name: "read".into(),
                arguments: r#"{"path":"src/main.rs"}"#.into(),
            },
        };
        let result = convert_tool_call_to_action(&tc).unwrap();
        assert!(matches!(result, AgentAction::ReadFile { path } if path == "src/main.rs"));
    }

    #[test]
    fn convert_provider_tool_call_to_shell_action() {
        let tc = crate::types_api::ToolCall {
            id: "call_2".into(),
            call_type: "function".into(),
            function: crate::types_api::ToolFunctionCall {
                name: "shell".into(),
                arguments: r#"{"command":"cargo test"}"#.into(),
            },
        };
        let result = convert_tool_call_to_action(&tc).unwrap();
        assert!(matches!(result, AgentAction::RunCommand { command } if command == "cargo test"));
    }

    #[test]
    fn convert_provider_tool_call_to_search_action() {
        let tc = crate::types_api::ToolCall {
            id: "call_3".into(),
            call_type: "function".into(),
            function: crate::types_api::ToolFunctionCall {
                name: "search".into(),
                arguments: r#"{"pattern":"fn main","path":"src"}"#.into(),
            },
        };
        let result = convert_tool_call_to_action(&tc).unwrap();
        assert!(
            matches!(result, AgentAction::SearchText { q, path } if q == "fn main" && path == "src")
        );
    }

    #[test]
    fn convert_provider_tool_call_to_done_action() {
        let tc = crate::types_api::ToolCall {
            id: "call_4".into(),
            call_type: "function".into(),
            function: crate::types_api::ToolFunctionCall {
                name: "respond".into(),
                arguments: r#"{"text":"Task complete."}"#.into(),
            },
        };
        let result = convert_tool_call_to_action(&tc).unwrap();
        assert!(matches!(result, AgentAction::Done { summary } if summary == "Task complete."));
    }

    #[test]
    fn convert_provider_tool_call_to_ask_action() {
        let tc = crate::types_api::ToolCall {
            id: "call_5".into(),
            call_type: "function".into(),
            function: crate::types_api::ToolFunctionCall {
                name: "ask".into(),
                arguments: r#"{"question":"What is your goal?"}"#.into(),
            },
        };
        let result = convert_tool_call_to_action(&tc).unwrap();
        assert!(
            matches!(result, AgentAction::Ask { question } if question == "What is your goal?")
        );
    }

    #[test]
    fn convert_provider_tool_call_rejects_unknown_name() {
        let tc = crate::types_api::ToolCall {
            id: "call_6".into(),
            call_type: "function".into(),
            function: crate::types_api::ToolFunctionCall {
                name: "nonexistent_tool".into(),
                arguments: r#"{}"#.into(),
            },
        };
        assert!(convert_tool_call_to_action(&tc).is_none());
    }

    #[test]
    fn convert_provider_tool_call_rejects_malformed_json() {
        let tc = crate::types_api::ToolCall {
            id: "call_7".into(),
            call_type: "function".into(),
            function: crate::types_api::ToolFunctionCall {
                name: "read".into(),
                arguments: r#"not valid json"#.into(),
            },
        };
        assert!(convert_tool_call_to_action(&tc).is_none());
    }

    #[test]
    fn convert_provider_tool_call_to_edit_action() {
        let tc = crate::types_api::ToolCall {
            id: "call_8".into(),
            call_type: "function".into(),
            function: crate::types_api::ToolFunctionCall {
                name: "edit".into(),
                arguments: r#"{"path":"file.txt","old":"foo","new":"bar"}"#.into(),
            },
        };
        let result = convert_tool_call_to_action(&tc).unwrap();
        assert!(
            matches!(result, AgentAction::EditFile { path, old, new } if path == "file.txt" && old == "foo" && new == "bar")
        );
    }
}
