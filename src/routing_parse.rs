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

pub(crate) fn parse_json_loose<T: DeserializeOwned>(text: &str) -> Result<T> {
    if let Ok(v) = serde_json::from_str::<T>(text.trim()) {
        return Ok(v);
    }
    if let Some(obj) = extract_first_json_object(text) {
        return serde_json::from_str::<T>(obj.trim())
            .context("Failed to parse extracted JSON object");
    }
    anyhow::bail!("No JSON object found")
}
