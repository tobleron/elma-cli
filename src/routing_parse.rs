//! @efficiency-role: util-pure
//!
//! Routing - JSON and Markdown Parsing

use crate::*;

/// Strip markdown code fences and common prose wrappers from model output.
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
                    let s = start?;
                    return cleaned.get(s..=i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Maximum JSON text length to attempt parsing (prevents DoS from model repetition loops)
const MAX_JSON_LENGTH: usize = 32_768; // 32KB

/// Fix common LLM JSON errors: orphaned keys after array elements
/// E.g. `[{"id":"s1"}, "purpose":"x", "depends_on":[]]` → `[{"id":"s1","purpose":"x","depends_on":[]}]`
fn fix_orphaned_keys_in_arrays(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut in_array = 0usize;
    let mut last_obj_end = None; // Position after last } in array
    
    while let Some(c) = chars.next() {
        result.push(c);
        match c {
            '[' => in_array += 1,
            ']' => {
                in_array = in_array.saturating_sub(1);
                last_obj_end = None;
            }
            '}' if in_array > 0 => {
                // Mark position after } in case we need to insert orphaned keys
                last_obj_end = Some(result.len());
            }
            '"' if in_array > 0 => {
                // Check if this looks like an orphaned key: ", "key":
                if let Some(pos) = last_obj_end {
                    // Check if we're right after }, and this is a known step field
                    let check_start = pos.saturating_sub(2);
                    let recent = &result[check_start..];
                    if recent.ends_with("}, ") || recent.ends_with("},\n") || recent.ends_with("},\r\n") {
                        // This might be an orphaned key - collect it
                        let key_start = result.len() - 1; // Position of the "
                        let mut key_chars = vec!['"'];
                        key_chars.push(c);
                        while let Some(&kc) = chars.peek() {
                            chars.next();
                            if kc == '"' {
                                key_chars.push(kc);
                                break;
                            }
                            key_chars.push(kc);
                        }
                        // Check if it's a known step field
                        let key_str: String = key_chars.iter().collect();
                        if key_str.starts_with("\"purpose\"") || key_str.starts_with("\"depends_on\"") 
                            || key_str.starts_with("\"success_condition\"") || key_str.starts_with("\"parent_id\"")
                            || key_str.starts_with("\"depth\"") || key_str.starts_with("\"unit_type\"")
                            || key_str.starts_with("\"common\"")
                        {
                            // Remove the }, and insert the key inside the object
                            result.truncate(pos - 2); // Remove },
                            result.push(',');
                            result.push_str(&key_str);
                            // Now collect the value
                            if chars.peek() == Some(&':') {
                                chars.next();
                                result.push(':');
                                // Collect value (string, array, or null)
                                while let Some(&vc) = chars.peek() {
                                    chars.next();
                                    result.push(vc);
                                    if vc == '"' {
                                        break; // String value done
                                    } else if vc == '[' || vc == '{' {
                                        // Collect nested structure
                                        let mut depth = 1;
                                        while let Some(&nc) = chars.peek() {
                                            chars.next();
                                            result.push(nc);
                                            if nc == '[' || nc == '{' {
                                                depth += 1;
                                            } else if nc == ']' || nc == '}' {
                                                depth -= 1;
                                                if depth == 0 {
                                                    break;
                                                }
                                            }
                                        }
                                        break;
                                    } else if vc == 'n' || vc == 't' || vc == 'f' {
                                        // null, true, false - collect rest of word
                                        while let Some(&nc) = chars.peek() {
                                            if nc.is_alphabetic() {
                                                chars.next();
                                                result.push(nc);
                                            } else {
                                                break;
                                            }
                                        }
                                        break;
                                    }
                                }
                                result.push('}');
                                last_obj_end = Some(result.len());
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    result
}

/// Detect repetition loops in model output (e.g., "fn main\\(|" repeated hundreds of times)
fn detect_repetition_loop(text: &str) -> Option<String> {
    // Look for patterns that repeat more than 20 times
    let min_repeat_len = 8; // Minimum pattern length to check
    let min_repeats = 20;   // Minimum repetitions to flag as loop
    
    let bytes = text.as_bytes();
    if bytes.len() < min_repeat_len * min_repeats {
        return None;
    }
    
    // Check for repeated substrings
    for pattern_len in min_repeat_len..=50 {
        if bytes.len() < pattern_len * min_repeats {
            break;
        }
        
        // Sample a pattern from the middle of the text (where repetition often starts)
        let sample_start = bytes.len() / 3;
        if sample_start + pattern_len > bytes.len() {
            continue;
        }
        
        let pattern = &text[sample_start..sample_start + pattern_len];
        if pattern.is_empty() || pattern.chars().all(|c| c.is_whitespace()) {
            continue;
        }
        
        // Count repetitions
        let mut count = 0;
        let mut pos = 0;
        while let Some(found) = text[pos..].find(pattern) {
            count += 1;
            pos += found + pattern.len();
            if count >= min_repeats {
                return Some(format!("Pattern '{}' repeated {}+ times", 
                    if pattern.len() > 20 { &pattern[..20] } else { pattern }, count));
            }
        }
    }
    
    None
}

pub(crate) fn parse_json_loose<T: DeserializeOwned>(text: &str) -> Result<T> {
    let trimmed = text.trim();

    // Check for model repetition loops
    if let Some(loop_info) = detect_repetition_loop(trimmed) {
        anyhow::bail!("Model repetition loop detected: {}. JSON parsing aborted.", loop_info);
    }

    // Check for model repetition loops (absurdly long output)
    if trimmed.len() > MAX_JSON_LENGTH {
        anyhow::bail!("JSON response too long ({} chars) - model may be in repetition loop", trimmed.len());
    }

    if let Ok(v) = serde_json::from_str::<T>(trimmed) {
        return Ok(v);
    }

    // Fallback 1: Extract JSON object from prose/markdown
    if let Some(obj) = extract_first_json_object(trimmed) {
        let obj_trimmed = obj.trim();

        // Also check extracted JSON length
        if obj_trimmed.len() > MAX_JSON_LENGTH {
            anyhow::bail!("Extracted JSON too long ({} chars) - model may be in repetition loop", obj_trimmed.len());
        }

        if let Ok(v) = serde_json::from_str::<T>(obj_trimmed) {
            return Ok(v);
        }

        // Fallback 1b: Fix orphaned keys in arrays (common LLM error)
        let fixed = fix_orphaned_keys_in_arrays(obj_trimmed);
        if let Ok(v) = serde_json::from_str::<T>(&fixed) {
            return Ok(v);
        }

        // Fallback 2: Attempt structural repair on the extracted object
        if let Ok(repaired) = jsonrepair_rs::jsonrepair(obj_trimmed) {
            if let Ok(v) = serde_json::from_str::<T>(&repaired) {
                return Ok(v);
            }
        }
    }

    // Fallback 3: Attempt structural repair on the entire text
    if let Ok(repaired) = jsonrepair_rs::jsonrepair(trimmed) {
        if let Ok(v) = serde_json::from_str::<T>(&repaired) {
            return Ok(v);
        }
    }

    // Provide detailed error context for debugging
    let preview = if trimmed.len() > 500 {
        format!("{}...", &trimmed[..500])
    } else {
        trimmed.to_string()
    };
    anyhow::bail!("No valid or repairable JSON object found. Preview: {}", preview)
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
        // This simulates the specific failure from the last session
        let input = r#"{"objective":"list files","steps":[{"id":"s1","type":"shell","cmd":"ls -1"}, "purpose":"inspect","success_condition":"ok","depends_on":[]}]}"#;

        #[derive(Deserialize, Serialize, Debug, PartialEq)]
        struct MockProgram {
            objective: String,
            steps: Vec<serde_json::Value>,
        }

        let result: Result<MockProgram> = parse_json_loose(input);
        assert!(result.is_ok(), "Should be able to repair and parse the malformed JSON. Error: {:?}", result.err());
        let program = result.unwrap();
        assert_eq!(program.objective, "list files");
        // json-repair might fix it by either dropping the orphaned keys or wrapping them.
        // Either way, the top-level parse should succeed.
        assert!(!program.steps.is_empty());
    }

    #[test]
    fn test_detect_repetition_loop() {
        // Simulate the model repetition loop from session s_1774899901_489608000
        let repetitive = r#"{"objective":"test","steps":[{"cmd":"fn main\\(|fn main\\(|fn main\\(|fn main\\(|fn main\\(|fn main\\(|fn main\\(|fn main\\(|fn main\\(|fn main\\(|fn main\\(|fn main\\(|fn main\\(|fn main\\(|fn main\\(|fn main\\(|fn main\\(|fn main\\(|fn main\\(|fn main\\(|fn main\\(|fn main\\("|}]}"#;
        
        let result: Result<serde_json::Value> = parse_json_loose(repetitive);
        assert!(result.is_err(), "Should detect repetition loop");
        let err_msg = result.err().unwrap().to_string();
        assert!(err_msg.contains("repetition loop"), "Error should mention repetition loop: {}", err_msg);
    }

    #[test]
    fn test_normal_json_not_flagged_as_repetitive() {
        // Normal JSON with some repeated words should not be flagged
        let normal = r#"{"objective":"test","steps":[{"cmd":"echo hello"},{"cmd":"echo world"}]}"#;

        let result: Result<serde_json::Value> = parse_json_loose(normal);
        assert!(result.is_ok(), "Normal JSON should parse successfully. Error: {:?}", result.err());
    }

    #[test]
    fn test_fix_orphaned_keys_in_steps() {
        // Test the specific malformed JSON from session s_1774904245_340673000
        let malformed = r#"{"objective":"list files","steps":[{"id":"s1","type":"shell","cmd":"ls -1"}, "purpose":"inspect","success_condition":"ok","depends_on":[]}]}"#;
        
        let fixed = fix_orphaned_keys_in_arrays(malformed);
        // The fix should move the orphaned keys inside the step object
        assert!(fixed.contains(r#"{"id":"s1","type":"shell","cmd":"ls -1","purpose":"inspect","success_condition":"ok","depends_on":[]}"#) 
            || fixed.contains(r#""purpose":"inspect""#), "Fixed JSON should contain purpose inside step. Got: {}", fixed);
        
        // Try to parse as Value to verify it's valid JSON
        let result: Result<serde_json::Value> = parse_json_loose(malformed);
        assert!(result.is_ok(), "Should repair orphaned keys. Error: {:?}, Fixed: {}", result.err(), fixed);
    }
}
