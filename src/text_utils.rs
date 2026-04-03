use crate::*;

pub(crate) fn looks_like_path_token(s: &str) -> bool {
    let t = s.trim_matches(|c: char| {
        matches!(
            c,
            '"' | '\'' | '`' | ',' | '.' | ';' | ':' | ')' | ']' | '}'
        )
    });
    if t.is_empty() {
        return false;
    }
    if t.contains('/') || t.contains('\\') {
        return true;
    }
    let lower = t.to_ascii_lowercase();
    lower.ends_with(".toml")
        || lower.ends_with(".md")
        || lower.ends_with(".rs")
        || lower.ends_with(".txt")
        || lower.ends_with(".json")
        || lower.ends_with(".lock")
        || lower == "makefile"
        || lower == "dockerfile"
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

    trimmed_tokens
        .iter()
        .copied()
        .find(|tok| tok.contains('/') || tok.contains('\\'))
        .map(str::to_string)
        .or_else(|| {
            trimmed_tokens
                .iter()
                .copied()
                .find(|tok| looks_like_path_token(tok))
                .map(str::to_string)
        })
        .or_else(|| {
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
    // POSIX-ish single-quote escaping: ' -> '\''.
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for ch in s.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
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
}
