//! @efficiency-role: util-pure
//!
//! Evidence Summary Module
//!
//! Immediate summarization of tool results into compact factual statements.
//! Raw output goes to disk; one-line summaries go into the ledger and narrative.

use crate::*;

const SMALL_OUTPUT_THRESHOLD: usize = 500;

pub(crate) fn summarize_tool_result(
    tool_name: &str,
    raw_output: &str,
    extra: &SummarizeExtra,
) -> String {
    if raw_output.len() < SMALL_OUTPUT_THRESHOLD {
        return raw_output.trim().to_string();
    }

    match tool_name {
        "shell" => summarize_shell(raw_output, extra),
        "read" => summarize_read(raw_output, extra),
        "search" => summarize_search(raw_output, extra),
        _ => summarize_generic(tool_name, raw_output),
    }
}

pub(crate) struct SummarizeExtra {
    pub(crate) command: Option<String>,
    pub(crate) path: Option<String>,
    pub(crate) pattern: Option<String>,
    pub(crate) exit_code: Option<i32>,
}

fn summarize_shell(raw: &str, extra: &SummarizeExtra) -> String {
    let cmd = extra.command.as_deref().unwrap_or("unknown");
    let lines = raw.lines().count();
    let bytes = raw.len();
    let exit = extra
        .exit_code
        .map(|c| if c == 0 { "ok" } else { "failed" })
        .unwrap_or("unknown");

    let first_line = raw.lines().next().unwrap_or("").trim();
    let key_finding = if first_line.len() > 120 {
        &first_line[..120]
    } else {
        first_line
    };

    format!("shell: {cmd} → {exit}, {lines} lines, {bytes}B. Key: {key_finding}")
}

fn summarize_read(raw: &str, extra: &SummarizeExtra) -> String {
    let path = extra.path.as_deref().unwrap_or("unknown");
    let lines = raw.lines().count();
    let bytes = raw.len();

    format!("read: {path} → {lines} lines, {bytes}B")
}

fn summarize_search(raw: &str, extra: &SummarizeExtra) -> String {
    let pattern = extra.pattern.as_deref().unwrap_or("unknown");
    let path = extra.path.as_deref().unwrap_or("");
    let matches = raw.lines().filter(|l| !l.trim().is_empty()).count();

    let location = if path.is_empty() {
        "workspace".to_string()
    } else {
        format!("in {path}")
    };

    format!("search: {pattern} {location} → {matches} matches")
}

fn summarize_generic(tool_name: &str, raw: &str) -> String {
    let lines = raw.lines().count();
    let bytes = raw.len();
    format!("{tool_name} → {lines} lines, {bytes}B")
}

pub(crate) fn should_store_raw(raw_output: &str) -> bool {
    raw_output.len() >= SMALL_OUTPUT_THRESHOLD
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_output_returns_raw() {
        let small = "hello world";
        let extra = SummarizeExtra {
            command: None,
            path: None,
            pattern: None,
            exit_code: None,
        };
        let result = summarize_tool_result("shell", small, &extra);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_large_shell_output_summarized() {
        let large = "line1\nline2\nline3\n".repeat(200);
        let extra = SummarizeExtra {
            command: Some("ls -la".to_string()),
            path: None,
            pattern: None,
            exit_code: Some(0),
        };
        let result = summarize_tool_result("shell", &large, &extra);
        assert!(result.contains("ls -la"));
        assert!(result.contains("ok"));
        assert!(result.contains("lines"));
    }

    #[test]
    fn test_large_read_output_summarized() {
        let large = "code line\n".repeat(200);
        let extra = SummarizeExtra {
            command: None,
            path: Some("src/main.rs".to_string()),
            pattern: None,
            exit_code: None,
        };
        let result = summarize_tool_result("read", &large, &extra);
        assert!(result.contains("src/main.rs"));
        assert!(result.contains("lines"));
        assert!(result.contains("B"));
    }

    #[test]
    fn test_large_search_output_summarized() {
        let large = "file.rs:10: fn foo() {}\n".repeat(50);
        let extra = SummarizeExtra {
            command: None,
            path: Some("src/".to_string()),
            pattern: Some("fn foo".to_string()),
            exit_code: None,
        };
        let result = summarize_tool_result("search", &large, &extra);
        assert!(result.contains("fn foo"));
        assert!(result.contains("50 matches"));
    }

    #[test]
    fn test_should_store_raw_threshold() {
        assert!(!should_store_raw("small"));
        assert!(should_store_raw(&"x".repeat(500)));
        assert!(should_store_raw(&"x".repeat(1000)));
    }

    #[test]
    fn test_generic_summarize() {
        let large = "data\n".repeat(200);
        let result = summarize_generic("custom_tool", &large);
        assert!(result.contains("custom_tool"));
        assert!(result.contains("200 lines"));
    }
}
