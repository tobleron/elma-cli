//! @efficiency-role: ui-component
//!
//! Terminal Markdown Renderer — Claude Code Style
//!
//! Renders markdown matching Claude Code's rendering:
//! - `# H1` → bold + italic + underline
//! - `## H2`+ → bold
//! - `**bold**` → bold
//! - `_italic_` / `*italic*` → italic
//! - `` `code` `` → permission color (purple)
//! - `> quote` → `▎` prefix + italic
//! - `- item` → bullet list with `•`
//! - `1. item` → numbered list
//! - `---` → horizontal rule
//! - Code fences → syntax highlighted, no borders
//!
//! No truncation — full output always rendered.

use crate::ui_colors::*;
use crate::ui_syntax::*;
use crate::ui_layout::BLOCKQUOTE_BAR;

/// Render markdown text to terminal-formatted output.
pub(crate) fn render_markdown(text: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut in_code_block = false;
    let mut code_lang = String::new();
    let mut code_buffer = String::new();

    for raw_line in text.lines() {
        // Code block fences
        if raw_line.trim().starts_with("```") {
            if in_code_block {
                // End of code block — syntax highlight, no borders
                let highlighted = if code_lang.is_empty() {
                    highlight_auto(&code_buffer)
                } else {
                    highlight_code(&code_buffer, &code_lang)
                };

                // Claude Code does NOT put borders around code blocks
                // Just render the highlighted code directly
                for hl_line in highlighted.lines() {
                    lines.push(hl_line.to_string());
                }
                lines.push(String::new()); // spacer after code

                code_buffer.clear();
                code_lang.clear();
                in_code_block = false;
            } else {
                in_code_block = true;
                let lang = raw_line.trim().strip_prefix("```").unwrap_or("").trim();
                if !lang.is_empty() {
                    code_lang = lang.to_string();
                }
            }
            continue;
        }

        if in_code_block {
            code_buffer.push_str(raw_line);
            code_buffer.push('\n');
            continue;
        }

        let trimmed = raw_line.trim();

        // Horizontal rule
        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            lines.push(crate::ui_layout::render_hr());
            lines.push(String::new());
            continue;
        }

        // Headers — match Claude Code:
        // h1 = bold + italic + underline
        // h2+ = bold
        if let Some(rest) = trimmed.strip_prefix("###### ") {
            lines.push(format!("      {}", warn_yellow(&format!("\x1b[1m{}\x1b[22m", rest))));
            lines.push(String::new());
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("##### ") {
            lines.push(format!("     {}", warn_yellow(&format!("\x1b[1m{}\x1b[22m", rest))));
            lines.push(String::new());
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("#### ") {
            lines.push(format!("    {}", warn_yellow(&format!("\x1b[1m{}\x1b[22m", rest))));
            lines.push(String::new());
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("### ") {
            lines.push(format!("  {}", warn_yellow(&format!("\x1b[1m{}\x1b[22m", rest))));
            lines.push(String::new());
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("## ") {
            lines.push(warn_yellow(&format!("\x1b[1m{}\x1b[22m", rest)));
            lines.push(String::new());
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("# ") {
            lines.push(render_h1(rest));
            lines.push(String::new());
            continue;
        }

        // Blockquote — Claude Code uses `▎` + italic
        if let Some(rest) = trimmed.strip_prefix("> ") {
            lines.push(format!("{} {}", meta_comment(BLOCKQUOTE_BAR), meta_comment(&format!("*{}*", rest))));
            continue;
        }
        if trimmed.starts_with('>') {
            let rest = trimmed.strip_prefix('>').unwrap_or("").trim();
            lines.push(format!("{} {}", meta_comment(BLOCKQUOTE_BAR), meta_comment(&format!("*{}*", rest))));
            continue;
        }

        // Unordered list items
        if let Some(rest) = trimmed.strip_prefix("- ") {
            lines.push(format!("{} {}", info_cyan("•"), render_inline_md(rest)));
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("* ") {
            lines.push(format!("{} {}", info_cyan("•"), render_inline_md(rest)));
            continue;
        }

        // Ordered list items
        if let Some(pos) = trimmed.find(". ") {
            let prefix = &trimmed[..pos];
            if prefix.chars().all(|c| c.is_ascii_digit()) && prefix.len() <= 5 {
                let rest = &trimmed[pos + 2..];
                lines.push(format!("{} {}", warn_yellow(&format!("{}.", prefix)), render_inline_md(rest)));
                continue;
            }
        }

        // Blank line
        if trimmed.is_empty() {
            lines.push(String::new());
            continue;
        }

        // Regular paragraph text — render inline markdown
        lines.push(render_inline_md(trimmed));
    }

    // Handle unclosed code block
    if in_code_block && !code_buffer.is_empty() {
        let highlighted = if code_lang.is_empty() {
            highlight_auto(&code_buffer)
        } else {
            highlight_code(&code_buffer, &code_lang)
        };
        for hl_line in highlighted.lines() {
            lines.push(hl_line.to_string());
        }
    }

    lines.join("\n")
}

/// Render H1: bold + italic + underline
fn render_h1(text: &str) -> String {
    // In terminal, we simulate underline by wrapping with ANSI codes
    // \x1b[1m = bold, \x1b[3m = italic, \x1b[4m = underline
    warn_yellow(&format!("\x1b[1m\x1b[3m\x1b[4m{}\x1b[0m", text))
}

/// Render bold text
fn render_bold(text: &str) -> String {
    text_white(&format!("\x1b[1m{}\x1b[22m", text))
}

/// Render inline markdown formatting.
fn render_inline_md(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            // Inline code: `code` — Claude Code uses "permission" color (purple)
            '`' => {
                let mut code = String::new();
                while let Some(&c) = chars.peek() {
                    chars.next();
                    if c == '`' { break; }
                    code.push(c);
                }
                result.push_str(&elma_accent(&format!("`{}`", code)));
            }
            // Bold: **text**
            '*' => {
                if chars.peek() == Some(&'*') {
                    chars.next();
                    let mut bold = String::new();
                    let mut found_end = false;
                    while let Some(c) = chars.next() {
                        if c == '*' && chars.peek() == Some(&'*') {
                            chars.next();
                            found_end = true;
                            break;
                        } else {
                            bold.push(c);
                        }
                    }
                    if found_end {
                        result.push_str(&render_bold(&bold));
                    } else {
                        result.push_str(&format!("**{}", bold));
                    }
                } else {
                    // Single * = italic
                    let mut italic = String::new();
                    while let Some(c) = chars.next() {
                        if c == '*' { break; }
                        italic.push(c);
                    }
                    result.push_str(&meta_comment(&format!("*{}*", italic)));
                }
            }
            // Italic/bold with underscores
            '_' => {
                if chars.peek() == Some(&'_') {
                    chars.next();
                    let mut bold = String::new();
                    let mut found_end = false;
                    while let Some(c) = chars.next() {
                        if c == '_' && chars.peek() == Some(&'_') {
                            chars.next();
                            found_end = true;
                            break;
                        } else {
                            bold.push(c);
                        }
                    }
                    if found_end {
                        result.push_str(&render_bold(&bold));
                    } else {
                        result.push_str(&format!("__{}", bold));
                    }
                } else {
                    // Single _ = italic
                    let mut italic = String::new();
                    while let Some(c) = chars.next() {
                        if c == '_' { break; }
                        italic.push(c);
                    }
                    result.push_str(&meta_comment(&format!("_{}_", italic)));
                }
            }
            _ => result.push(ch),
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_simple_text() {
        let output = render_markdown("Hello world");
        assert!(output.contains("Hello world"));
    }

    #[test]
    fn test_render_h1() {
        let output = render_markdown("# Title");
        assert!(output.contains("Title"));
        assert!(output.contains("\x1b[1m")); // bold
        assert!(output.contains("\x1b[4m")); // underline
    }

    #[test]
    fn test_render_h2() {
        let output = render_markdown("## Title");
        assert!(output.contains("Title"));
        assert!(output.contains("\x1b[1m")); // bold
    }

    #[test]
    fn test_render_code_block() {
        let output = render_markdown("```rust\nfn main() {}\n```");
        assert!(output.contains("fn"));
        // Claude Code does NOT put borders around code
        assert!(!output.contains("─"));
    }

    #[test]
    fn test_render_horizontal_rule() {
        let output = render_markdown("---");
        assert!(output.contains("─"));
    }

    #[test]
    fn test_render_list() {
        let output = render_markdown("- item one\n- item two");
        assert!(output.contains("•"));
        assert!(output.contains("item one"));
    }

    #[test]
    fn test_render_blockquote() {
        let output = render_markdown("> This is a quote");
        assert!(output.contains(BLOCKQUOTE_BAR));
        assert!(output.contains("This is a quote"));
    }

    #[test]
    fn test_render_inline_code() {
        let output = render_markdown("Use `println!()` for output");
        assert!(output.contains("`println!()`"));
        // Should be in Mauve (Catppuccin accent)
        assert!(output.contains("\x1b[38;2;211;134;155m"));
    }

    #[test]
    fn test_render_bold() {
        let output = render_markdown("This is **bold** text");
        assert!(output.contains("bold"));
        assert!(output.contains("\x1b[1m"));
    }

    #[test]
    fn test_no_truncation() {
        let long_text = (0..200).map(|i| format!("Line {}\n", i)).collect::<Vec<_>>().join("\n");
        let output = render_markdown(&long_text);
        let line_count = output.lines().count();
        assert!(line_count >= 190);
    }
}
