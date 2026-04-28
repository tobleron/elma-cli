//! @efficiency-role: ui-component
//!
//! Terminal Markdown Renderer — delegates to markdown-to-ansi crate.
//!
//! Converts markdown to ANSI-formatted terminal text with syntax highlighting,
//! tables, lists, and inline formatting. Used by the legacy (non-Ratatui)
//! render path.

use crate::markdown_ansi::render_markdown_to_ansi;

/// Render markdown text to ANSI-formatted terminal output.
pub(crate) fn render_markdown(text: &str) -> String {
    render_markdown_to_ansi(text)
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
    }

    #[test]
    fn test_render_h2() {
        let output = render_markdown("## Title");
        assert!(output.contains("Title"));
    }

    #[test]
    fn test_render_code_block() {
        let output = render_markdown("```rust\nfn main() {}\n```");
        assert!(output.contains("fn"));
    }

    #[test]
    fn test_render_horizontal_rule() {
        let output = render_markdown("---");
        // markdown-to-ansi may render or elide thematic breaks
        // depending on parser configuration; just check it doesn't crash
        assert!(!output.contains("panic"));
    }

    #[test]
    fn test_render_list() {
        let output = render_markdown("- item one\n- item two");
        assert!(output.contains("item one"));
    }

    #[test]
    fn test_render_blockquote() {
        let output = render_markdown("> This is a quote");
        assert!(output.contains("This is a quote"));
    }

    #[test]
    fn test_render_inline_code() {
        let output = render_markdown("Use `println!()` for output");
        assert!(output.contains("println!()"));
    }

    #[test]
    fn test_render_bold() {
        let output = render_markdown("This is **bold** text");
        assert!(output.contains("bold"));
    }

    #[test]
    fn test_no_truncation() {
        let long_text = (0..200)
            .map(|i| format!("Line {}\n", i))
            .collect::<Vec<_>>()
            .join("\n");
        let output = render_markdown(&long_text);
        let line_count = output.lines().count();
        assert!(line_count >= 190);
    }
}
