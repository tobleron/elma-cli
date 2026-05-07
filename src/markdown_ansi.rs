//! @efficiency-role: infra-adapter
//!
//! Markdown-to-ANSI rendering via the markdown-to-ansi crate.
//!
//! Converts markdown text to terminal ANSI escape sequences for display
//! in non-Ratatui paths (stdout printing, legacy renderer, session files).
//!
//! **ANSI conversion is ONLY for external command output and legacy stdout paths.**
//! LLM Markdown → Ratatui must go through the structured pipeline in claude_markdown.rs
//! (parse_markdown → RenderBlock IR → render_blocks_to_lines).
//! No code path should call `render_markdown_to_ansi` and then re-parse ANSI back into
//! Ratatui spans — that would corrupt the structured intermediate representation.
//!
//! Task 700: --no-color support. When no_color is set, strip ANSI from rendered output.

use crate::*;
use markdown_to_ansi::Options;
use std::sync::{OnceLock, RwLock};

static NO_COLOR_FLAG: OnceLock<RwLock<bool>> = OnceLock::new();

pub(crate) fn no_color_enabled() -> bool {
    if let Ok(lock) = NO_COLOR_FLAG.get_or_init(|| RwLock::new(false)).read() {
        *lock
    } else {
        false
    }
}

/// Set whether ANSI color output should be suppressed (Task 700).
pub(crate) fn set_no_color(enabled: bool) {
    if let Ok(mut lock) = NO_COLOR_FLAG.get_or_init(|| RwLock::new(false)).write() {
        *lock = enabled;
    }
}

fn default_options() -> Options {
    Options {
        syntax_highlight: !no_color_enabled(),
        width: None,
        code_bg: !no_color_enabled(),
    }
}

/// Render markdown to ANSI-formatted terminal text.
/// When --no-color is active, strips ANSI escape sequences from the output (Task 700).
pub(crate) fn render_markdown_to_ansi(text: &str) -> String {
    if no_color_enabled() {
        let rendered = markdown_to_ansi::render(text, &default_options());
        strip_ansi_escapes::strip(rendered.as_bytes())
            .map(|b| String::from_utf8_lossy(&b).to_string())
            .unwrap_or(rendered)
    } else {
        markdown_to_ansi::render(text, &default_options())
    }
}

/// Render inline markdown (no block-level elements) to ANSI.
/// When --no-color is active, strips ANSI escape sequences from the output (Task 700).
pub(crate) fn render_markdown_inline_to_ansi(text: &str) -> String {
    if no_color_enabled() {
        let rendered = markdown_to_ansi::render_inline(text, &default_options());
        strip_ansi_escapes::strip(rendered.as_bytes())
            .map(|b| String::from_utf8_lossy(&b).to_string())
            .unwrap_or(rendered)
    } else {
        markdown_to_ansi::render_inline(text, &default_options())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_headers() {
        let output = render_markdown_to_ansi("# Hello World");
        assert!(output.contains("Hello World"));
    }

    #[test]
    fn test_render_bold() {
        let output = render_markdown_to_ansi("**bold** text");
        assert!(output.contains("bold"));
    }

    #[test]
    fn test_render_code_block() {
        let output = render_markdown_to_ansi("```rust\nfn main() {}\n```");
        // markdown-to-ansi with syntax highlighting strips backticks but
        // includes the code content with ANSI formatting
        assert!(!output.contains("```"));
        assert!(output.contains("main"));
    }

    #[test]
    fn test_render_inline() {
        let output = render_markdown_inline_to_ansi("**bold** and *italic*");
        assert!(output.contains("bold"));
        assert!(output.contains("italic"));
    }
}
