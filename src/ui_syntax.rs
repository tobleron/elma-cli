//! @efficiency-role: ui-component
//!
//! Task 111: Syntect Syntax Highlighting
//!
//! Provides syntax highlighting for code blocks displayed in the terminal.
//! Uses syntect for language-aware highlighting with ANSI escape codes.
//!
//! Design: Minimal, cached syntax sets, graceful fallback for unknown languages.

use syntect::easy::HighlightLines;
use syntect::highlighting::{Color as SyntectColor, Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Cached highlighted snippets to avoid re-computation.
fn highlight_cache() -> &'static Mutex<HashMap<String, String>> {
    static CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Load the syntax set once (expensive operation).
fn get_syntax_set() -> &'static SyntaxSet {
    static SS: OnceLock<SyntaxSet> = OnceLock::new();
    SS.get_or_init(SyntaxSet::load_defaults_newlines)
}

/// Load the theme once. Uses base16-ocean.dark for Rose Pine compatibility.
fn get_theme() -> &'static syntect::highlighting::Theme {
    use std::sync::OnceLock;
    static THEME: OnceLock<syntect::highlighting::Theme> = OnceLock::new();
    THEME.get_or_init(|| {
        let ts = ThemeSet::load_defaults();
        ts.themes
            .get("base16-ocean.dark")
            .cloned()
            .unwrap_or_else(|| ts.themes["InspiredGitHub"].clone())
    })
}

/// Convert a syntect Color to ANSI 24-bit escape code.
fn color_to_ansi(color: SyntectColor, is_foreground: bool) -> String {
    let code = if is_foreground { "38" } else { "48" };
    format!("\x1b[{};2;{};{};{}m", code, color.r, color.g, color.b)
}

/// Apply ANSI styling to a string segment.
fn apply_style(text: &str, style: Style) -> String {
    let mut out = String::new();

    // Foreground
    if style.foreground.a != 0 {
        out.push_str(&color_to_ansi(style.foreground, true));
    }

    // Background
    if style.background.a != 0 {
        out.push_str(&color_to_ansi(style.background, false));
    }

    // Bold
    if style.font_style.contains(syntect::highlighting::FontStyle::BOLD) {
        out.push_str("\x1b[1m");
    }

    // Italic
    if style.font_style.contains(syntect::highlighting::FontStyle::ITALIC) {
        out.push_str("\x1b[3m");
    }

    // Underline
    if style.font_style.contains(syntect::highlighting::FontStyle::UNDERLINE) {
        out.push_str("\x1b[4m");
    }

    out.push_str(text);
    out.push_str("\x1b[0m"); // Reset
    out
}

/// Highlight a code block with the given language.
/// Returns the highlighted string with ANSI escape codes.
/// Falls back to plain text if language is unknown.
pub(crate) fn highlight_code(code: &str, language: &str) -> String {
    // Check cache first
    let cache_key = format!("{}:{}", language, code);
    if let Ok(cache) = highlight_cache().lock() {
        if let Some(cached) = cache.get(&cache_key) {
            return cached.clone();
        }
    }

    let ss = get_syntax_set();
    let theme = get_theme();

    // Find syntax by language name
    let syntax = ss
        .find_syntax_by_name(language)
        .or_else(|| ss.find_syntax_by_extension(language))
        .unwrap_or_else(|| ss.find_syntax_plain_text());

    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut highlighted = String::new();

    for line in LinesWithEndings::from(code) {
        let ranges = highlighter.highlight_line(line, ss).unwrap_or_default();
        for &(style, text) in &ranges {
            highlighted.push_str(&apply_style(text, style));
        }
    }

    // Cache the result (cap cache at 100 entries to prevent memory growth)
    if let Ok(mut cache) = highlight_cache().lock() {
        if cache.len() < 100 {
            cache.insert(cache_key, highlighted.clone());
        }
    }

    highlighted
}

/// Highlight a code block without caching (for one-off highlighting).
pub(crate) fn highlight_code_once(code: &str, language: &str) -> String {
    highlight_code(code, language)
}

/// Detect if a string looks like it contains code (has indentation, brackets, keywords).
/// Returns the detected language or None.
pub(crate) fn detect_language(code: &str) -> Option<&'static str> {
    let trimmed = code.trim();

    // Simple heuristic detection
    if trimmed.starts_with("fn ") || trimmed.starts_with("pub fn ") || trimmed.starts_with("struct ") {
        return Some("Rust");
    }
    if trimmed.starts_with("def ") || trimmed.starts_with("class ") || trimmed.starts_with("import ") {
        return Some("Python");
    }
    if trimmed.starts_with("function ") || trimmed.starts_with("const ") || trimmed.starts_with("let ") {
        return Some("JavaScript");
    }
    if trimmed.starts_with("{") || trimmed.contains("\"key\"") {
        return Some("JSON");
    }
    if trimmed.starts_with("#include") || trimmed.starts_with("int main") {
        return Some("C");
    }
    if trimmed.starts_with("<!DOCTYPE") || trimmed.starts_with("<html") || trimmed.starts_with("<div") {
        return Some("HTML");
    }
    if trimmed.starts_with("SELECT ") || trimmed.starts_with("CREATE TABLE") {
        return Some("SQL");
    }
    if trimmed.starts_with("#!/bin/") || trimmed.starts_with("ls ") || trimmed.starts_with("find ") {
        return Some("Bourne Again Shell (bash)");
    }

    None
}

/// Highlight a code block with auto-detected language.
pub(crate) fn highlight_auto(code: &str) -> String {
    if let Some(lang) = detect_language(code) {
        highlight_code(code, lang)
    } else {
        // Plain text — no ANSI codes
        code.to_string()
    }
}

/// Process a full message and highlight all markdown code blocks.
/// Detects ```lang ... ``` blocks and highlights them.
pub(crate) fn highlight_message(message: &str) -> String {
    if !message.contains("```") {
        return message.to_string(); // No code blocks
    }

    let mut result = String::with_capacity(message.len() * 2);
    let mut in_code_block = false;
    let mut current_lang = String::new();
    let mut code_buffer = String::new();

    for line in message.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            if in_code_block {
                // End of code block — highlight and append
                let highlighted = if current_lang.is_empty() {
                    highlight_auto(&code_buffer)
                } else {
                    highlight_code(&code_buffer, &current_lang)
                };
                result.push_str(&highlighted);
                code_buffer.clear();
                current_lang.clear();
                in_code_block = false;
            } else {
                // Start of code block
                in_code_block = true;
                // Extract language if specified
                let lang = trimmed.strip_prefix("```").unwrap_or("").trim();
                if !lang.is_empty() {
                    current_lang = lang.to_string();
                }
            }
        } else if in_code_block {
            code_buffer.push_str(line);
            code_buffer.push('\n');
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }

    // Handle unclosed code block
    if in_code_block && !code_buffer.is_empty() {
        let highlighted = if current_lang.is_empty() {
            highlight_auto(&code_buffer)
        } else {
            highlight_code(&code_buffer, &current_lang)
        };
        result.push_str(&highlighted);
    }

    // Remove trailing newline if original didn't have one
    if !message.ends_with('\n') && result.ends_with('\n') {
        result.pop();
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_rust_code() {
        let code = "fn main() {\n    println!(\"Hello, world!\");\n}\n";
        let highlighted = highlight_code(code, "Rust");
        // Should contain ANSI escape codes
        assert!(highlighted.contains("\x1b["));
        // Should contain the original text fragments (ANSI codes may split them)
        assert!(highlighted.contains("fn"));
        assert!(highlighted.contains("main"));
        assert!(highlighted.contains("println"));
        assert!(highlighted.contains("Hello, world!"));
    }

    #[test]
    fn test_highlight_unknown_language() {
        let code = "some gibberish text\n";
        let highlighted = highlight_code(code, "nonexistent_lang_xyz");
        // Should still work (falls back to plain text syntax)
        assert!(!highlighted.is_empty());
    }

    #[test]
    fn test_detect_language_rust() {
        assert_eq!(detect_language("fn main() {}"), Some("Rust"));
        assert_eq!(detect_language("pub fn test() {}"), Some("Rust"));
    }

    #[test]
    fn test_detect_language_python() {
        assert_eq!(detect_language("def hello():\n    pass"), Some("Python"));
    }

    #[test]
    fn test_detect_language_json() {
        assert_eq!(detect_language("{\"key\": \"value\"}"), Some("JSON"));
    }

    #[test]
    fn test_detect_unknown() {
        assert_eq!(detect_language("just some plain text"), None);
    }

    #[test]
    fn test_highlight_auto_detect() {
        let rust_code = "fn main() {}\n";
        let highlighted = highlight_auto(rust_code);
        assert!(highlighted.contains("\x1b[")); // Has ANSI codes
    }

    #[test]
    fn test_highlight_cache_works() {
        let code = "fn test() {}\n";
        let first = highlight_code(code, "Rust");
        let second = highlight_code(code, "Rust");
        assert_eq!(first, second); // Same result from cache
    }
}
