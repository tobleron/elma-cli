use crate::*;

pub(crate) fn looks_like_path_token(s: &str) -> bool {
    let t = s.trim_matches(|c: char| c == '"' || c == '\'' || c == '`');
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

pub(crate) fn extract_first_path_from_user_text(line: &str) -> Option<String> {
    for tok in line.split_whitespace() {
        if looks_like_path_token(tok) {
            return Some(
                tok.trim_matches(|c: char| c == '"' || c == '\'' || c == '`')
                    .to_string(),
            );
        }
    }
    None
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
