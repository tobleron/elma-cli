//! @efficiency-role: util-pure
//!

use crate::*;
use once_cell::sync::Lazy;
use regex::Regex;

// Pre-compiled regex patterns for performance
static PATH_TOKEN_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r#"[^\s"'`;:(){}\[\]\/\\]+/[^\s"'`;:(){}\[\]\/\\]*|[^\s"'`;:(){}\[\]\/\\]+\\[^\s"'`;:(){}\[\]\/\\]*"#).unwrap());
static FILE_EXTENSION_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r#"(?i)\.(toml|md|rs|txt|json|lock)$"#).unwrap());

/// Strip `...</thinking>` blocks that
/// leak from models even when reasoning_format=none.
pub(crate) fn strip_thinking_blocks(text: &str) -> String {
    let mut result = text.to_string();
    loop {
        if let Some(start) = result.find("<think>") {
            if let Some(end) = result.find("</think>") {
                result.replace_range(start..end + "</think>".len(), "");
                continue;
            }
            // Unclosed <think> — strip to end of text
            result.replace_range(start.., "");
            continue;
        }
        if let Some(start) = result.find("<thinking>") {
            if let Some(end) = result.find("</thinking>") {
                result.replace_range(start..end + "</thinking>".len(), "");
                continue;
            }
            result.replace_range(start.., "");
            continue;
        }
        if let Some(start) = result.find("<reasoning>") {
            if let Some(end) = result.find("</reasoning>") {
                result.replace_range(start..end + "</reasoning>".len(), "");
                continue;
            }
            result.replace_range(start.., "");
            continue;
        }
        break;
    }
    loop {
        if let Some(start) = result.find("<tool_call>") {
            if let Some(end) = result.find("</tool_call>") {
                result.replace_range(start..end + "</tool_call>".len(), "");
                continue;
            }
        }
        break;
    }
    result.trim().to_string()
}

pub(crate) fn looks_like_path_token(s: &str) -> bool {
    // Trim common punctuation
    let t = s.trim_matches(|c: char| {
        matches!(
            c,
            '"' | '\'' | '`' | ',' | '.' | ';' | ':' | ')' | ']' | '}'
        )
    });
    
    if t.is_empty() {
        return false;
    }
    
    // Check if it contains path separators
    if t.contains('/') || t.contains('\\') {
        return true;
    }
    
    // Check if it ends with known file extensions
    let lower = t.to_ascii_lowercase();
    FILE_EXTENSION_REGEX.is_match(&lower) || 
        lower == "makefile" || 
        lower == "dockerfile"
}

fn existing_workspace_token(s: &str) -> Option<String> {
    let t = s.trim_matches(|c: char| {
        matches!(
            c,
            '"' | '\'' | '`' | ',' | '.' | ';' | ':' | ')' | ']' | '}'
        )
    });
    if t.is_empty() || t.starts_with('-') {
        return None;
    }
    if t.contains('/') || t.contains('\\') {
        return None;
    }
    let candidate = std::path::Path::new(t);
    if candidate.exists() {
        return Some(t.to_string());
    }
    None
}

pub(crate) fn extract_first_path_from_user_text(line: &str) -> Option<String> {
    // Trim common punctuation from tokens and look for path patterns
    let trimmed_tokens = line
        .split_whitespace()
        .map(|tok| {
            tok.trim_matches(|c: char| {
                matches!(
                    c,
                    '"' | '\'' | '`' | ',' | '.' | ';' | ':' | ')' | ']' | '}'
                )
            })
        })
        .filter(|tok| !tok.is_empty())
        .collect::<Vec<_>>();

    // First try to find tokens that look like paths using regex
    trimmed_tokens
        .iter()
        .copied()
        .find(|tok| PATH_TOKEN_REGEX.is_match(tok))
        .map(str::to_string)
        .or_else(|| {
            // Then try to find tokens that look like path tokens (files, etc.)
            trimmed_tokens
                .iter()
                .copied()
                .find(|tok| looks_like_path_token(tok))
                .map(str::to_string)
        })
        .or_else(|| {
            // Finally check for existing workspace tokens
            trimmed_tokens
                .iter()
                .find_map(|tok| existing_workspace_token(tok))
        })
}

pub(crate) fn plain_terminal_text(s: &str) -> String {
    // Minimal "de-markdown" for terminal readability:
    // - remove code fences
    // - strip backticks
    // - convert leading "* " bullets to "- "
    // - drop heading markers
    let mut out = String::new();
    let mut in_fence = false;
    for raw in s.lines() {
        let line = raw.trim_end();
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            out.push_str(line);
            out.push('\n');
            continue;
        }
        let mut l = line.to_string();
        if l.trim_start().starts_with('#') {
            l = l.trim_start_matches('#').trim_start().to_string();
        }
        if let Some(rest) = l.strip_prefix("* ") {
            l = format!("- {rest}");
        }
        l = l.replace('`', "");
        // Remove simple emphasis markers.
        l = l.replace("**", "");
        l = l.replace('*', "");
        out.push_str(l.trim_end());
        out.push('\n');
    }
    squash_blank_lines(out.trim()).trim().to_string()
}

pub(crate) fn shell_quote(s: &str) -> String {
    shlex::quote(s).to_string()
}

pub(crate) fn normalize_shell_cmd(cmd: &str) -> String {
    let c = cmd.trim();
    // Common flaky model output: "ls -" (dangling flag).
    if c == "ls -" || c.ends_with(" ls -") || c.ends_with("\nls -") {
        return "ls -l".to_string();
    }
    if c.starts_with("ls -") && c.len() <= "ls -".len() + 2 && c.ends_with('-') {
        return "ls -l".to_string();
    }
    // Another common: "cat cargo.toml" wrong casing on macOS.
    if c.starts_with("cat cargo.toml") {
        return c.replacen("cat cargo.toml", "cat Cargo.toml", 1);
    }
    if c.starts_with("rg ") {
        let tokens: Vec<&str> = c.split_whitespace().collect();
        if let Some(globstar_token) = tokens.iter().find(|token| token.contains("/**/*")) {
            if let Some((base, glob)) = globstar_token.split_once("/**/") {
                let normalized_glob = if glob.is_empty() {
                    "*".to_string()
                } else {
                    glob.to_string()
                };
                let mut rewritten = tokens
                    .iter()
                    .filter(|token| **token != *globstar_token)
                    .map(|token| (*token).to_string())
                    .collect::<Vec<_>>();
                if !base.is_empty() {
                    rewritten.push(base.to_string());
                }
                rewritten.push("--glob".to_string());
                rewritten.push(shell_quote(&normalized_glob));
                return rewritten.join(" ");
            }
        }
    }
    c.to_string()
}

pub(crate) fn summarize_shell_output(output: &str) -> String {
    const MAX_CHARS: usize = 12_000;
    let trimmed = output.trim();
    if trimmed.len() <= MAX_CHARS {
        return trimmed.to_string();
    }
    let mut s = trimmed[..MAX_CHARS].to_string();
    s.push_str("\n[truncated]");
    s
}

pub(crate) fn looks_like_markdown(text: &str) -> bool {
    let t = text.trim();
    t.contains("```")
        || t.contains('`')
        || t.lines().any(|l| l.trim_start().starts_with("#"))
        || t.lines().any(|l| l.trim_start().starts_with("* "))
}

pub(crate) fn user_requested_markdown(text: &str) -> bool {
    let t = text.to_lowercase();
    t.contains("markdown")
}

/// Collapse markdown formatting to compact plain text suitable for TUI display.
/// Removes heading markers, bold/italic markers, horizontal rules, and collapses
/// excessive blank lines.
pub(crate) fn compact_plain_text(text: &str) -> String {
    let mut result = text.to_string();

    let heading_re = Regex::new(r"^#{1,6}\s+").unwrap();
    result = heading_re.replace_all(&result, "").to_string();

    let bold_re = Regex::new(r"\*\*(.+?)\*\*").unwrap();
    result = bold_re.replace_all(&result, "$1").to_string();

    let italic_re = Regex::new(r"\*(.+?)\*").unwrap();
    result = italic_re.replace_all(&result, "$1").to_string();

    let rule_re = Regex::new(r"^\s*-{3,}\s*$").unwrap();
    result = rule_re.replace_all(&result, "").to_string();

    let blank_re = Regex::new(r"\n{3,}").unwrap();
    result = blank_re.replace_all(&result, "\n\n").to_string();

    result.trim().to_string()
}

/// Normalize a shell command string for repeated-command detection.
/// Collapses large digit sequences (timestamps, session IDs) to `#`
/// while preserving small numbers used in limits (head -20, tail -50).
/// Also normalizes session path patterns to a stable marker.
pub fn normalize_shell_signal(cmd: &str) -> String {
    let mut out = String::with_capacity(cmd.len());
    let mut current_number = String::new();

    for ch in cmd.chars() {
        if ch.is_ascii_digit() {
            current_number.push(ch);
        } else {
            if !current_number.is_empty() {
                if current_number.len() >= 4 {
                    out.push('#');
                } else {
                    out.push_str(&current_number);
                }
                current_number.clear();
            }
            out.push(ch);
        }
    }
    if !current_number.is_empty() {
        if current_number.len() >= 4 {
            out.push('#');
        } else {
            out.push_str(&current_number);
        }
    }

    // Normalize session path patterns (s_1234_5678 → s_SESSION)
    let re = regex::Regex::new(r"s_#+").unwrap();
    re.replace_all(&out, "s_SESSION").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_first_path_trims_trailing_punctuation() {
        let line = "In _stress_testing/_opencode_for_testing/, find a function definition.";
        assert_eq!(
            extract_first_path_from_user_text(line).as_deref(),
            Some("_stress_testing/_opencode_for_testing/")
        );
    }

    #[test]
    fn extract_first_path_prefers_scoped_directory_over_filename() {
        let line =
            "Read the README.md in _stress_testing/_opencode_for_testing/ and create a summary.";
        assert_eq!(
            extract_first_path_from_user_text(line).as_deref(),
            Some("_stress_testing/_opencode_for_testing/")
        );
    }

    #[test]
    fn normalize_shell_cmd_rewrites_rg_globstar_path() {
        let original = "rg -i '^main' _stress_testing/_opencode_for_testing/**/*.rs";
        let normalized = normalize_shell_cmd(original);
        assert_eq!(
            normalized,
            "rg -i '^main' _stress_testing/_opencode_for_testing --glob '*.rs'"
        );
    }

    #[test]
    fn extract_first_path_detects_existing_workspace_directory_token() {
        let line = "umm can u pls list src and dont overdo it";
        assert_eq!(
            extract_first_path_from_user_text(line).as_deref(),
            Some("src")
        );
    }

    #[test]
    fn strip_thinking_blocks_removes_think_tags() {
        let raw = "<think>\ninternal reasoning\n</think>\nactual answer";
        assert_eq!(strip_thinking_blocks(raw), "actual answer");
    }

    #[test]
    fn strip_thinking_blocks_removes_tool_call_tags() {
        let raw = "<tool_call>{\"name\":\"respond\"}</tool_call>\nanswer";
        assert_eq!(strip_thinking_blocks(raw), "answer");
    }

    #[test]
    fn strip_thinking_blocks_handles_nested_blocks() {
        let raw = "<think>think1</think>\n<think>think2</think>\nanswer";
        assert_eq!(strip_thinking_blocks(raw), "answer");
    }

    #[test]
    fn strip_thinking_blocks_passes_through_clean_text() {
        let raw = "just a normal answer";
        assert_eq!(strip_thinking_blocks(raw), "just a normal answer");
    }
}
