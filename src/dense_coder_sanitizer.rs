//! @efficiency-role: util-pure
//! Sanitizes output from dense coder models that emit interleaved content/tool calls
//! without proper delimiters. Task 644.

use regex::Regex;

/// State maintained across streaming deltas.
#[derive(Debug, Clone, Default)]
pub(crate) struct SanitizerState {
    pub in_tool_block: bool,
    pub in_content_block: bool,
    pub buffer: String,
    pub tool_call_count: u32,
}

/// A raw tool call extracted from mixed content.
#[derive(Debug, Clone)]
pub(crate) struct RawToolCall {
    pub name: String,
    pub input: String,
    pub start_pos: usize,
    pub end_pos: usize,
}

/// Heuristic: detects whether a text delta contains both natural-language text
/// and tool-call markup (JSON blocks, XML function tags, or bracket markers).
pub(crate) fn detect_mixed_content(text: &str) -> bool {
    let has_text = text.len() > 10;
    let has_tool_markup = text.contains("```json")
        || text.contains("<function_call>")
        || text.contains("<function_result>")
        || text.contains("[Tool call:")
        || text.contains("\"name\":")
        || text.contains("\"arguments\":");
    has_text && has_tool_markup
}

/// Splits raw output into (artifact, final_answer). The artifact is everything
/// up to the last natural-language answer boundary; the answer is everything
/// after. Uses heuristics based on common dense-coder output patterns.
pub(crate) fn split_artifact_from_answer(text: &str) -> (String, String) {
    let trimmed = text.trim();

    // If the entire output is a single JSON tool call, there is no answer.
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if v.is_object() && v.get("name").is_some() && v.get("arguments").is_some() {
            return (trimmed.to_string(), String::new());
        }
    }

    // Find the last block of natural-language text that looks like an answer.
    // Common patterns: paragraphs starting after tool blocks, or lines without JSON/XML.
    let lines: Vec<&str> = trimmed.lines().collect();
    let mut answer_start = lines.len();

    for (i, line) in lines.iter().enumerate().rev() {
        let trimmed_line = line.trim();
        if trimmed_line.is_empty() {
            continue;
        }
        // If this line looks like answer text (not tool markup, not JSON),
        // consider it as part of the answer.
        let is_tool_markup = trimmed_line.starts_with('{')
            || trimmed_line.starts_with("```")
            || trimmed_line.starts_with("<function_call>")
            || trimmed_line.starts_with("<function_result>")
            || trimmed_line.starts_with("[Tool call:")
            || trimmed_line.starts_with("\"name\":")
            || trimmed_line.starts_with("\"arguments\":");
        if !is_tool_markup {
            answer_start = i;
        } else {
            break;
        }
    }

    if answer_start == 0 {
        (String::new(), trimmed.to_string())
    } else if answer_start >= trimmed.len() {
        (trimmed.to_string(), String::new())
    } else {
        let artifact = lines[..answer_start].join("\n");
        let answer = lines[answer_start..].join("\n");
        (artifact, answer)
    }
}

/// Main sanitizer for dense-coder output.
pub(crate) struct OutputSanitizer;

impl OutputSanitizer {
    /// Processes a streaming delta, updating state. Returns the sanitized delta
    /// (content that should be forwarded to the caller).
    pub(crate) fn sanitize_stream(raw_delta: &str, state: &mut SanitizerState) -> String {
        let mut result = String::new();

        // Detect entry into tool call blocks
        if raw_delta.contains("<function_call>") {
            state.in_tool_block = true;
            let before = raw_delta.split("<function_call>").next().unwrap_or("");
            let cleaned_before = Self::clean_tool_markers(before);
            if !cleaned_before.is_empty() {
                result.push_str(&cleaned_before);
            }
            // Buffer everything after the opening tag
            let after = raw_delta.split("<function_call>").nth(1).unwrap_or("");
            state.buffer.push_str(after);
            return result;
        }

        if raw_delta.contains("</function_call>") {
            let before = raw_delta.split("</function_call>").next().unwrap_or("");
            state.buffer.push_str(before);
            // Complete tool call — extract it
            let tool_json = Self::extract_json_from_buffer(&state.buffer);
            if let Some(json) = tool_json {
                result.push_str(&json);
            }
            state.buffer.clear();
            state.in_tool_block = false;
            state.tool_call_count += 1;
            // Content after the closing tag
            let after = raw_delta.split("</function_call>").nth(1).unwrap_or("");
            let cleaned_after = Self::clean_tool_markers(after);
            if !cleaned_after.is_empty() {
                if !result.is_empty() {
                    result.push(' ');
                }
                result.push_str(&cleaned_after);
            }
            return result;
        }

        if raw_delta.contains("<function_result>") {
            state.in_tool_block = true;
            let before = raw_delta.split("<function_result>").next().unwrap_or("");
            let cleaned_before = Self::clean_tool_markers(before);
            if !cleaned_before.is_empty() {
                result.push_str(&cleaned_before);
            }
            return result;
        }

        if raw_delta.contains("</function_result>") {
            state.in_tool_block = false;
            let after = raw_delta.split("</function_result>").nth(1).unwrap_or("");
            let cleaned_after = Self::clean_tool_markers(after);
            if !cleaned_after.is_empty() {
                result.push_str(&cleaned_after);
            }
            return result;
        }

        // Handle bracket-style tool call markers: [Tool call: name(args)]
        if raw_delta.contains("[Tool call:") {
            let re = Regex::new(r"\[Tool call: [^\]]+\]").unwrap();
            let cleaned = re.replace_all(raw_delta, "");
            let cleaned = cleaned.trim().to_string();
            if !cleaned.is_empty() {
                result.push_str(&cleaned);
            }
            state.tool_call_count += 1;
            return result;
        }

        // Handle inline JSON tool calls ({"name": "tool", "arguments": {...}})
        // mixed with natural text
        if state.in_tool_block {
            state.buffer.push_str(raw_delta);
            return String::new();
        }

        if detect_mixed_content(raw_delta) {
            let (content, tool_json) = Self::separate_json_tool_calls(raw_delta);
            if !content.trim().is_empty() {
                result.push_str(content.trim_end());
            }
            if !tool_json.is_empty() {
                if !result.is_empty() {
                    result.push('\n');
                }
                result.push_str(&tool_json);
            }
            return result;
        }

        // Default: pass through, cleaning tool markers
        Self::clean_tool_markers(raw_delta)
    }

    /// Processes final output, applying all sanitization rules.
    pub(crate) fn sanitize_final(raw: &str) -> String {
        let mut result = raw.to_string();

        // Remove [Tool call: ...] markers
        let re = Regex::new(r"\[Tool call: [^\]]+\]").unwrap();
        result = re.replace_all(&result, "").to_string();

        // Remove <function_call> ... </function_call> blocks
        let re = Regex::new(r"(?s)<function_call>.*?</function_call>").unwrap();
        result = re.replace_all(&result, "").to_string();

        // Remove <function_result> ... </function_result> blocks
        let re = Regex::new(r"(?s)<function_result>.*?</function_result>").unwrap();
        result = re.replace_all(&result, "").to_string();

        // Separate standalone JSON tool calls from preceding text
        let re = Regex::new(r#"\n\s*(\{\s*"name"\s*:)"#).unwrap();
        result = re.replace_all(&result, "\n\n$1").to_string();

        // Clean trailing whitespace from content lines
        let re = Regex::new(r"[ \t]+\n").unwrap();
        result = re.replace_all(&result, "\n").to_string();

        // Collapse multiple blank lines
        let re = Regex::new(r"\n{3,}").unwrap();
        result = re.replace_all(&result, "\n\n").to_string();

        result.trim().to_string()
    }

    /// Extracts tool calls from text that may contain mixed content.
    /// Returns parsed RawToolCall structs, deduplicated by position.
    pub(crate) fn extract_tool_calls(text: &str) -> Vec<RawToolCall> {
        let mut calls: Vec<RawToolCall> = Vec::new();

        // Pattern 1: <function_call> JSON </function_call> — most specific, process first
        let re = Regex::new(r"(?s)<function_call>\s*(\{.*?\})\s*</function_call>").unwrap();
        for cap in re.captures_iter(text) {
            let full_match = cap.get(0).unwrap();
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&cap[1]) {
                let name = val
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let input = val
                    .get("arguments")
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| cap[1].to_string());
                calls.push(RawToolCall {
                    name: name.to_string(),
                    input,
                    start_pos: full_match.start(),
                    end_pos: full_match.end(),
                });
            }
        }

        // Collect ranges claimed by more specific patterns so we can skip them.
        let claimed_ranges: Vec<(usize, usize)> =
            calls.iter().map(|c| (c.start_pos, c.end_pos)).collect();

        // Pattern 2: raw {"name": "...", "arguments": {...}} JSON (outside claimed ranges)
        let json_re = Regex::new(
            r#"\{\s*"name"\s*:\s*"([^"]+)"\s*,\s*"arguments"\s*:\s*(\{(?:[^{}]|\{(?:[^{}]|\{(?:[^{}]|\{[^{}]*\})*\})*\})*\})\s*\}"#
        ).unwrap();
        for cap in json_re.captures_iter(text) {
            let full_match = cap.get(0).unwrap();
            let start = full_match.start();
            let end = full_match.end();
            if claimed_ranges
                .iter()
                .any(|(cs, ce)| start >= *cs && end <= *ce)
            {
                continue;
            }
            calls.push(RawToolCall {
                name: cap[1].to_string(),
                input: cap[2].to_string(),
                start_pos: start,
                end_pos: end,
            });
        }

        // Update claimed ranges
        let claimed_ranges: Vec<(usize, usize)> =
            calls.iter().map(|c| (c.start_pos, c.end_pos)).collect();

        // Pattern 3: [Tool call: name(args)] markers
        let re = Regex::new(r"\[Tool call: (\w+)\(([^\]]*)\)\]").unwrap();
        for cap in re.captures_iter(text) {
            let full_match = cap.get(0).unwrap();
            let start = full_match.start();
            let end = full_match.end();
            if claimed_ranges
                .iter()
                .any(|(cs, ce)| start >= *cs && end <= *ce)
            {
                continue;
            }
            calls.push(RawToolCall {
                name: cap[1].to_string(),
                input: cap[2].to_string(),
                start_pos: start,
                end_pos: end,
            });
        }

        calls.sort_by_key(|c| c.start_pos);
        calls
    }

    // -- private helpers --

    fn clean_tool_markers(s: &str) -> String {
        let s = Regex::new(r"\[Tool call: [^\]]+\]")
            .unwrap()
            .replace_all(s, "");
        let s = Regex::new(r"(?s)<function_call>.*?</function_call>")
            .unwrap()
            .replace_all(&s, "");
        let s = Regex::new(r"(?s)<function_result>.*?</function_result>")
            .unwrap()
            .replace_all(&s, "");
        s.trim().to_string()
    }

    fn extract_json_from_buffer(buffer: &str) -> Option<String> {
        // Try to extract a JSON object from the buffer
        let start = buffer.find('{')?;
        let end = buffer.rfind('}')?;
        let candidate = &buffer[start..=end];
        if serde_json::from_str::<serde_json::Value>(candidate).is_ok() {
            Some(candidate.to_string())
        } else {
            None
        }
    }

    fn separate_json_tool_calls(text: &str) -> (String, String) {
        let re = Regex::new(
            r#"(\{\s*"name"\s*:\s*"([^"]+)"\s*,\s*"arguments"\s*:\s*(\{(?:[^{}]|\{(?:[^{}]|\{(?:[^{}]|\{[^{}]*\})*\})*\})*\})\s*\})"#
        ).unwrap();
        let mut content = text.to_string();
        let mut calls = Vec::new();
        for cap in re.captures_iter(text) {
            let m = cap.get(0).unwrap();
            calls.push((m.start(), m.end(), m.as_str().to_string()));
        }
        // Remove calls from content in reverse order
        for (start, end, _) in calls.iter().rev() {
            content.replace_range(*start..*end, "");
        }
        let call_json: String = calls
            .iter()
            .map(|(_, _, s)| s.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        (content, call_json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_text_passes_through_unchanged() {
        let text = "Hello, this is a normal response.";
        assert_eq!(
            OutputSanitizer::sanitize_final(text),
            "Hello, this is a normal response."
        );
    }

    #[test]
    fn removes_tool_call_bracket_markers() {
        let text = "Let me check that file. [Tool call: read(filePath=\"src/main.rs\")] The file contains...";
        let result = OutputSanitizer::sanitize_final(text);
        assert!(!result.contains("[Tool call:"));
        assert!(result.contains("Let me check that file."));
        assert!(result.contains("The file contains..."));
    }

    #[test]
    fn extract_tool_calls_from_mixed_content() {
        let text = r#"Some text {"name": "read", "arguments": {"filePath": "src/main.rs"}} more text {"name": "write", "arguments": {"path": "foo.txt", "content": "bar"}}"#;
        let calls = OutputSanitizer::extract_tool_calls(text);
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].name, "read");
        assert_eq!(calls[1].name, "write");
        assert!(calls[0].start_pos < calls[0].end_pos);
    }

    #[test]
    fn extract_tool_calls_from_function_call_tags() {
        let text = r#"<function_call>{"name": "read", "arguments": {"filePath": "test.txt"}}</function_call>"#;
        let calls = OutputSanitizer::extract_tool_calls(text);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "read");
    }

    #[test]
    fn extract_tool_calls_from_bracket_markers() {
        let text = "Some text [Tool call: read(src/main.rs)] more text";
        let calls = OutputSanitizer::extract_tool_calls(text);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "read");
    }

    #[test]
    fn split_artifact_from_answer_empty_artifact() {
        let text = "This is the final answer without any artifact.";
        let (artifact, answer) = split_artifact_from_answer(text);
        assert!(artifact.is_empty());
        assert_eq!(
            answer.trim(),
            "This is the final answer without any artifact."
        );
    }

    #[test]
    fn split_artifact_from_answer_with_json_tool_call() {
        let text = r#"{"name": "read", "arguments": {"filePath": "src/main.rs"}}
The file contains the main entry point."#;
        let (artifact, answer) = split_artifact_from_answer(text);
        assert!(!artifact.is_empty());
        assert_eq!(answer.trim(), "The file contains the main entry point.");
    }

    #[test]
    fn split_artifact_single_json_is_artifact_no_answer() {
        let text = r#"{"name": "read", "arguments": {"filePath": "src/main.rs"}}"#;
        let (artifact, answer) = split_artifact_from_answer(text);
        assert!(!artifact.is_empty());
        assert!(answer.is_empty());
    }

    #[test]
    fn detect_mixed_content_true_with_json() {
        assert!(detect_mixed_content("some text {\"name\": \"read\"}"));
    }

    #[test]
    fn detect_mixed_content_true_with_bracket_marker() {
        assert!(detect_mixed_content("some text [Tool call: read(x)]"));
    }

    #[test]
    fn detect_mixed_content_false_plain_text() {
        assert!(!detect_mixed_content("Just a normal sentence."));
    }

    #[test]
    fn streaming_state_transitions() {
        let mut state = SanitizerState::default();

        let delta1 = OutputSanitizer::sanitize_stream("Here is the result. ", &mut state);
        assert_eq!(delta1, "Here is the result.");
        assert!(!state.in_tool_block);

        let delta2 = OutputSanitizer::sanitize_stream("<function_call>", &mut state);
        assert!(state.in_tool_block);

        let delta3 = OutputSanitizer::sanitize_stream(
            r#"{"name": "read", "arguments": {"filePath": "x.txt"}}"#,
            &mut state,
        );
        assert!(state.in_tool_block);
        assert!(!state.buffer.is_empty());

        let delta4 = OutputSanitizer::sanitize_stream("</function_call>", &mut state);
        assert!(!state.in_tool_block);
        assert_eq!(state.tool_call_count, 1);
    }

    #[test]
    fn streaming_preserves_content_after_tool_block() {
        let mut state = SanitizerState::default();

        OutputSanitizer::sanitize_stream("start ", &mut state);
        OutputSanitizer::sanitize_stream("<function_call>{\"name\":\"x\"}", &mut state);
        let result = OutputSanitizer::sanitize_stream("</function_call> end", &mut state);
        assert!(result.contains("end"));
    }

    #[test]
    fn function_result_blocks_are_stripped() {
        let text = "Before <function_result>some data</function_result> After";
        let result = OutputSanitizer::sanitize_final(text);
        assert!(!result.contains("<function_result>"));
        assert!(!result.contains("some data"));
        assert!(result.contains("Before"));
        assert!(result.contains("After"));
    }

    #[test]
    fn trailing_whitespace_cleaned() {
        let text = "line1   \nline2  \nline3";
        let result = OutputSanitizer::sanitize_final(text);
        for line in result.lines() {
            assert!(!line.ends_with(' '));
        }
    }

    #[test]
    fn multiple_blank_lines_collapsed() {
        let text = "a\n\n\n\n\nb";
        let result = OutputSanitizer::sanitize_final(text);
        assert_eq!(result, "a\n\nb");
    }

    #[test]
    fn detect_mixed_content_xml_tags() {
        assert!(detect_mixed_content(
            "some text <function_call>...</function_call>"
        ));
    }
}
