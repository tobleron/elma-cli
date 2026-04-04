pub mod config;
pub mod rust;

#[derive(Default)]
pub struct CommonMetrics {
    pub loc: usize,
    pub logic_count: usize,
    pub max_nesting: usize,
    pub complexity_penalty: f64,
    pub hotspot_lines: Option<(usize, usize)>,
    pub hotspot_reason: Option<String>,
    pub hotspot_symbol: Option<String>,
    pub external_calls: usize,     // Imports/Opens
    pub internal_calls: usize,     // Local function calls
    pub state_count: usize,        // Mutable variables/state markers
    pub dependencies: Vec<String>, // Explicit imports/modules
}

pub enum EfficiencyOverride {
    None,
    Ignore,
    Strict,
    Role(String),
    SkipViolation(String),
}

pub fn parse_header(content: &str) -> EfficiencyOverride {
    let prefixes = [
        "@efficiency:",
        "@efficiency-role:",
        "@efficiency-role ",
        "@efficiency-skip-violation:",
    ];
    for prefix in &prefixes {
        if let Some(pos) = content.find(prefix) {
            let start = pos + prefix.len();
            let mut val = String::new();
            for c in content[start..].chars() {
                if c.is_alphanumeric() || c == '-' || c == '_' || c == '!' {
                    val.push(c);
                } else if !val.is_empty() {
                    break;
                }
            }

            let tag = val.trim();
            if prefix.contains("skip-violation") {
                return EfficiencyOverride::SkipViolation(tag.to_string());
            }
            if tag == "ignore" || tag == "ignored" {
                return EfficiencyOverride::Ignore;
            }
            if tag == "strict" {
                return EfficiencyOverride::Strict;
            }
            if tag == "singleton" {
                return EfficiencyOverride::Role("orchestrator".to_string());
            }
            if !tag.is_empty() {
                return EfficiencyOverride::Role(tag.to_string());
            }
        }
    }
    EfficiencyOverride::None
}

pub fn strip_code(content: &str) -> String {
    strip_code_modular(content, true)
}

pub fn strip_code_modular(content: &str, treat_single_quote_as_string: bool) -> String {
    let mut result = String::with_capacity(content.len());
    let chars: Vec<char> = content.chars().collect();
    let mut i = 0;

    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut in_string = false;
    let mut string_char = ' ';

    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).cloned();

        if in_line_comment {
            if c == '\n' {
                in_line_comment = false;
                result.push(c);
            }
        } else if in_block_comment {
            if c == '*' && next == Some('/') {
                in_block_comment = false;
                i += 1;
            }
        } else if in_string {
            if c == '\\' && i + 1 < chars.len() {
                i += 1;
            }
            else if c == string_char {
                in_string = false;
            }
        } else if c == '/' && next == Some('/') {
            in_line_comment = true;
            i += 1;
        } else if c == '/' && next == Some('*') {
            in_block_comment = true;
            i += 1;
        } else if c == '"' || (treat_single_quote_as_string && c == '\'') || c == '`' {
            in_string = true;
            string_char = c;
        } else {
            result.push(c);
        }
        i += 1;
    }
    result
}

pub fn strip_test_blocks_rust(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();

        // Detect #[cfg(test)] or #[cfg(test)] followed by mod
        if line.starts_with("#[cfg(test)]") || line.starts_with("#[cfg(test)]") {
            // Skip until we find the matching closing brace at nesting level 0
            i += 1;
            let mut depth = 0;
            let mut started = false;
            while i < lines.len() {
                let l = lines[i];
                for c in l.chars() {
                    if c == '{' {
                        depth += 1;
                        started = true;
                    } else if c == '}' {
                        depth -= 1;
                    }
                }
                if started && depth == 0 {
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }

        // Detect individual #[test] or #[tokio::test] functions
        if line.starts_with("#[test]") || line.starts_with("#[tokio::test]") {
            let attr_line = line.clone();
            // Skip the attribute line(s) and find the function
            i += 1;
            while i < lines.len() {
                let l = lines[i].trim();
                if l.starts_with("#[") {
                    i += 1;
                    continue;
                }
                break;
            }
            // Now skip the function body (find opening brace, then track depth)
            let mut depth = 0;
            let mut started = false;
            // The function signature may be on the current line
            for l_idx in i..lines.len() {
                let l = lines[l_idx];
                for c in l.chars() {
                    if c == '{' {
                        depth += 1;
                        started = true;
                    } else if c == '}' {
                        depth -= 1;
                    }
                }
                if started && depth == 0 {
                    i = l_idx + 1;
                    break;
                }
            }
            if !started {
                // No brace found, just skip a few lines as safety
                i = i.saturating_add(1);
            }
            continue;
        }

        result.push_str(lines[i]);
        result.push('\n');
        i += 1;
    }
    result
}

pub fn apply_complexity_dictionary(
    content: &str,
    dict: &std::collections::HashMap<String, f64>,
) -> f64 {
    let mut penalty = 0.0;
    for (pattern, weight) in dict {
        penalty += (content.matches(pattern).count() as f64) * weight;
    }
    penalty
}
