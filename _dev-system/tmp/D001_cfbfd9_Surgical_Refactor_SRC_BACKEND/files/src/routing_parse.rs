//! @efficiency-role: util-pure
//!
//! Routing - JSON and Markdown Parsing

use crate::*;

pub(crate) fn strip_markdown_wrappers(text: &str) -> &str {
    let mut result = text;
    if let Some(fence_start) = result.find("```") {
        let after_fence = &result[fence_start + 3..];
        let lang_end = after_fence.find('\n').unwrap_or(0);
        result = &result[fence_start + 3 + lang_end..];
    }
    if let Some(fence_end) = result.rfind("```") {
        result = &result[..fence_end];
    }
    result.trim()
}

pub(crate) fn extract_first_json_object(text: &str) -> Option<&str> {
    let cleaned = strip_markdown_wrappers(text);
    let bytes = cleaned.as_bytes();
    let mut start = None;
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    for (i, &b) in bytes.iter().enumerate() {
        if start.is_none() {
            if b == b'{' {
                start = Some(i);
                depth = 1;
            }
            continue;
        }
        if in_string {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                in_string = false;
            }
            continue;
        }
        match b {
            b'"' => in_string = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return cleaned.get(start?..=i);
                }
            }
            _ => {}
        }
    }
    None
}

const MAX_JSON_LENGTH: usize = 32_768;

fn is_known_step_field(key_str: &str) -> bool {
    [
        "\"purpose\"",
        "\"depends_on\"",
        "\"success_condition\"",
        "\"parent_id\"",
        "\"depth\"",
        "\"unit_type\"",
        "\"common\"",
    ]
    .iter()
    .any(|&k| key_str.starts_with(k))
}

fn is_orphaned_key_boundary(recent: &str) -> bool {
    recent.ends_with("}, ") || recent.ends_with("},\n") || recent.ends_with("},\r\n")
}

fn collect_json_string_value(
    chars: &mut std::iter::Peekable<std::str::Chars>,
    result: &mut String,
) {
    while let Some(&vc) = chars.peek() {
        chars.next();
        result.push(vc);
        if vc == '"' {
            break;
        } else if matches!(vc, '[' | '{') {
            collect_nested_structure(chars, result);
            break;
        } else if matches!(vc, 'n' | 't' | 'f') {
            collect_literal_value(chars, result);
            break;
        }
    }
}

fn collect_nested_structure(chars: &mut std::iter::Peekable<std::str::Chars>, result: &mut String) {
    let mut depth = 1;
    while let Some(&nc) = chars.peek() {
        chars.next();
        result.push(nc);
        if matches!(nc, '[' | '{') {
            depth += 1;
        } else if matches!(nc, ']' | '}') {
            depth -= 1;
            if depth == 0 {
                break;
            }
        }
    }
}

fn collect_literal_value(chars: &mut std::iter::Peekable<std::str::Chars>, result: &mut String) {
    while let Some(&nc) = chars.peek() {
        if nc.is_alphabetic() {
            chars.next();
            result.push(nc);
        } else {
            break;
        }
    }
}

fn try_absorb_orphaned_key(
    result: &mut String,
    chars: &mut std::iter::Peekable<std::str::Chars>,
    obj_end_pos: usize,
) -> bool {
    let recent = &result[obj_end_pos.saturating_sub(2)..];
    if !is_orphaned_key_boundary(recent) {
        return false;
    }

    result.pop(); // remove trailing "
    let mut key_chars = String::from("\"");
    while let Some(&kc) = chars.peek() {
        chars.next();
        key_chars.push(kc);
        if kc == '"' {
            break;
        }
    }
    if !is_known_step_field(&key_chars) {
        result.push('"');
        return false;
    }

    result.truncate(obj_end_pos - 2);
    result.push(',');
    result.push_str(&key_chars);

    if chars.peek() == Some(&':') {
        chars.next();
        result.push(':');
        collect_json_string_value(chars, result);
        result.push('}');
    }
    true
}

fn fix_orphaned_keys_in_arrays(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut in_array = 0usize;
    let mut last_obj_end = None;

    while let Some(c) = chars.next() {
        result.push(c);
        match c {
            '[' => in_array += 1,
            ']' => {
                in_array = in_array.saturating_sub(1);
                last_obj_end = None;
            }
            '}' if in_array > 0 => {
                last_obj_end = Some(result.len());
            }
            '"' if in_array > 0 => {
                if let Some(pos) = last_obj_end {
                    if try_absorb_orphaned_key(&mut result, &mut chars, pos) {
                        last_obj_end = Some(result.len());
                    }
                }
            }
            _ => {}
        }
    }
    result
}

fn detect_repetition_loop(text: &str) -> Option<String> {
    const MIN_REPEAT_LEN: usize = 8;
    const MIN_REPEATS: usize = 20;
    let bytes = text.as_bytes();
    if bytes.len() < MIN_REPEAT_LEN * MIN_REPEATS {
        return None;
    }

    for pattern_len in MIN_REPEAT_LEN..=50 {
        if bytes.len() < pattern_len * MIN_REPEATS {
            break;
        }
        let sample_start = bytes.len() / 3;
        if sample_start + pattern_len > bytes.len() {
            continue;
        }
        let pattern: String = text.chars().skip(sample_start).take(pattern_len).collect();
        if pattern.is_empty() || pattern.chars().all(|c| c.is_whitespace()) {
            continue;
        }

        let mut count = 0;
        let mut pos = 0;
        while let Some(found) = text[pos..].find(&pattern) {
            count += 1;
            pos += found + pattern.len();
            if count >= MIN_REPEATS {
                let preview: String = pattern.chars().take(20).collect();
                return Some(format!("Pattern '{}' repeated {}+ times", preview, count));
            }
        }
    }
    None
}

fn validate_no_repetition_loop(trimmed: &str) -> Result<()> {
    if let Some(loop_info) = detect_repetition_loop(trimmed) {
        anyhow::bail!(
            "Model repetition loop detected: {}. JSON parsing aborted.",
            loop_info
        );
    }
    if trimmed.len() > MAX_JSON_LENGTH {
        anyhow::bail!(
            "JSON response too long ({} chars) - model may be in repetition loop",
            trimmed.len()
        );
    }
    Ok(())
}

fn try_parse_extracted_json<T: DeserializeOwned + 'static>(obj: &str) -> Result<T> {
    let obj_trimmed = obj.trim();
    if obj_trimmed.len() > MAX_JSON_LENGTH {
        anyhow::bail!(
            "Extracted JSON too long ({} chars) - model may be in repetition loop",
            obj_trimmed.len()
        );
    }
    if let Ok(v) = crate::json_parser::parse_with_repair::<T>(obj_trimmed) {
        return Ok(v);
    }
    let fixed = fix_orphaned_keys_in_arrays(obj_trimmed);
    if let Ok(v) = crate::json_parser::parse_with_repair::<T>(&fixed) {
        return Ok(v);
    }
    if let Ok(repaired) = jsonrepair_rs::jsonrepair(obj_trimmed) {
        if let Ok(v) = crate::json_parser::parse_with_repair::<T>(&repaired) {
            return Ok(v);
        }
    }
    anyhow::bail!("extracted JSON parse failed")
}

fn try_repair_and_parse<T: DeserializeOwned + 'static>(text: &str) -> Result<T> {
    if let Ok(repaired) = jsonrepair_rs::jsonrepair(text) {
        if let Ok(v) = crate::json_parser::parse_with_repair::<T>(&repaired) {
            return Ok(v);
        }
    }
    let preview = text.chars().take(500).collect::<String>();
    let preview = if text.chars().count() > 500 {
        format!("{}...", preview)
    } else {
        preview
    };
    anyhow::bail!(
        "No valid or repairable JSON object found. Preview: {}",
        preview
    )
}

pub(crate) fn parse_json_loose<T: DeserializeOwned + 'static>(text: &str) -> Result<T> {
    let trimmed = text.trim();
    validate_no_repetition_loop(trimmed)?;

    if let Ok(v) = crate::json_parser::parse_with_repair::<T>(trimmed) {
        return Ok(v);
    }
    if let Some(obj) = extract_first_json_object(trimmed) {
        if let Ok(v) = try_parse_extracted_json::<T>(obj) {
            return Ok(v);
        }
    }
    try_repair_and_parse::<T>(trimmed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_markdown_wrappers_removes_code_fences() {
        let input = r#"Here is a valid JSON object:

```json
{"objective": "test", "steps": []}
```"#;
        let result = strip_markdown_wrappers(input);
        assert!(result.starts_with('{'));
        assert!(result.ends_with('}'));
        assert!(!result.contains("```"));
    }

    #[test]
    fn strip_markdown_wrappers_handles_no_fences() {
        let input = r#"{"objective": "test", "steps": []}"#;
        let result = strip_markdown_wrappers(input);
        assert_eq!(result, input);
    }

    #[test]
    fn strip_markdown_wrappers_handles_prose_before_fence() {
        let input = r#"Here is the JSON you requested:

```
{"key": "value"}
```

Hope this helps!"#;
        let result = strip_markdown_wrappers(input);
        assert!(result.starts_with('{'));
        assert!(result.ends_with('}'));
    }

    #[test]
    fn extract_json_from_markdown_wrapped() {
        let input = r#"Here is a valid JSON object that matches the target schema:

```json
{
  "objective": "understand current project",
  "steps": [
    {"id": "s1", "type": "shell", "cmd": "cat Cargo.toml"}
  ]
}
```"#;
        let json = extract_first_json_object(input);
        assert!(json.is_some());
        let json_str = json.unwrap();
        assert!(json_str.starts_with('{'));
        assert!(json_str.ends_with('}'));
        let parsed: serde_json::Value = serde_json::from_str(json_str).unwrap();
        assert!(parsed.get("objective").is_some());
        assert!(parsed.get("steps").is_some());
    }

    #[test]
    fn extract_json_from_pure_json() {
        let input = r#"{"objective": "test", "steps": []}"#;
        let json = extract_first_json_object(input);
        assert!(json.is_some());
        assert_eq!(json.unwrap(), input);
    }

    #[test]
    fn extract_json_with_prose_after() {
        let input = r#"Here is a valid JSON object that matches the target schema:

```
{
  "objective": "understand current project",
  "steps": [
    {"id": "s1", "type": "shell", "cmd": "cat Cargo.toml"}
  ]
}
```

This JSON object has the following properties:
- "objective": This is the main objective.
- "steps": This is an array of steps."#;
        let json = extract_first_json_object(input);
        assert!(json.is_some());
        let json_str = json.unwrap();
        assert!(json_str.starts_with('{'));
        assert!(json_str.ends_with('}'));
        let parsed: serde_json::Value = serde_json::from_str(json_str).unwrap();
        assert!(parsed.get("objective").is_some());
    }

    #[test]
    fn test_repair_malformed_llm_json() {
        let input = r#"{"objective":"list files","steps":[{"id":"s1","type":"shell","cmd":"ls -1"}, "purpose":"inspect","success_condition":"ok","depends_on":[]}]}"#;

        #[derive(Deserialize, Serialize, Debug, PartialEq)]
        struct MockProgram {
            objective: String,
            steps: Vec<serde_json::Value>,
        }

        let result: Result<MockProgram> = parse_json_loose(input);
        assert!(
            result.is_ok(),
            "Should be able to repair and parse the malformed JSON. Error: {:?}",
            result.err()
        );
        let program = result.unwrap();
        assert_eq!(program.objective, "list files");
        assert!(!program.steps.is_empty());
    }

    #[test]
    fn test_detect_repetition_loop() {
        let repetitive = r#"{"objective":"test","steps":[{"cmd":"fn main\\(|fn main\\(|fn main\\(|fn main\\(|fn main\\(|fn main\\(|fn main\\(|fn main\\(|fn main\\(|fn main\\(|fn main\\(|fn main\\(|fn main\\(|fn main\\(|fn main\\(|fn main\\(|fn main\\(|fn main\\(|fn main\\(|fn main\\(|fn main\\(|fn main\\("|}]}"#;

        let result: Result<serde_json::Value> = parse_json_loose(repetitive);
        assert!(result.is_err(), "Should detect repetition loop");
        let err_msg = result.err().unwrap().to_string();
        assert!(
            err_msg.contains("repetition loop"),
            "Error should mention repetition loop: {}",
            err_msg
        );
    }

    #[test]
    fn test_normal_json_not_flagged_as_repetitive() {
        let normal = r#"{"objective":"test","steps":[{"cmd":"echo hello"},{"cmd":"echo world"}]}"#;
        let result: Result<serde_json::Value> = parse_json_loose(normal);
        assert!(
            result.is_ok(),
            "Normal JSON should parse successfully. Error: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_fix_orphaned_keys_in_steps() {
        let malformed = r#"{"objective":"list files","steps":[{"id":"s1","type":"shell","cmd":"ls -1"}, "purpose":"inspect","success_condition":"ok","depends_on":[]}]}"#;

        let fixed = fix_orphaned_keys_in_arrays(malformed);
        assert!(fixed.contains(r#"{"id":"s1","type":"shell","cmd":"ls -1","purpose":"inspect","success_condition":"ok","depends_on":[]}"#)
            || fixed.contains(r#""purpose":"inspect""#), "Fixed JSON should contain purpose inside step. Got: {}", fixed);

        let result: Result<serde_json::Value> = parse_json_loose(malformed);
        assert!(
            result.is_ok(),
            "Should repair orphaned keys. Error: {:?}, Fixed: {}",
            result.err(),
            fixed
        );
    }

    #[test]
    fn test_parse_error_preview_handles_unicode() {
        let input = "─".repeat(600);
        let result = std::panic::catch_unwind(|| {
            let _: Result<serde_json::Value> = parse_json_loose(&input);
        });
        assert!(result.is_ok());
    }
}
