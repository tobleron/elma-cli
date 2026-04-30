//! Input sanitization for untrusted model output.
//!
//! Before any DSL parsing begins, the raw model text must be sanitized:
//! - Strip ANSI escape sequences and control characters.
//! - Reject embedded NUL bytes.
//! - Optionally normalize CRLF to LF (used for block body content).
//! - Produce bounded debug previews that escape unprintable characters.

use crate::dsl::error::{DslError, DslErrorCode, DslResult, ParseContext};

/// Maximum length of debug preview text.
const DEBUG_PREVIEW_MAX: usize = 120;

/// Strip ANSI escape sequences and control characters from input.
///
/// Returns a sanitized String suitable for DSL parsing. Control characters are
/// removed; ANSI escape sequences (CSI, OSC, etc.) are stripped via the
/// `strip-ansi-escapes` crate. NUL bytes cause immediate rejection.
pub fn sanitize_control(raw: &str, context: &ParseContext) -> DslResult<String> {
    if raw.contains('\0') {
        return Err(DslError::new(
            DslErrorCode::NulByte,
            context.clone(),
            debug_preview(raw),
        ));
    }

    let bytes = raw.as_bytes();
    let stripped = match strip_ansi_escapes::strip(bytes) {
        Ok(v) => v,
        Err(_) => {
            // If stripping fails, fall back to manual control-char removal.
            let filtered: Vec<u8> = bytes
                .iter()
                .copied()
                .filter(|&b| {
                    b.is_ascii_graphic() || b == b'\n' || b == b'\r' || b == b' ' || b == b'\t'
                })
                .collect();
            filtered
        }
    };

    let out = String::from_utf8_lossy(&stripped)
        .chars()
        .filter(|ch| !ch.is_control() || *ch == '\n' || *ch == '\r' || *ch == '\t')
        .collect::<String>();
    Ok(out)
}

/// Strip ANSI via the crate; used in paths that already have their own NUL checks.
pub fn strip_ansi_for_dsl(raw: &[u8]) -> Vec<u8> {
    strip_ansi_escapes::strip(raw).unwrap_or_else(|_| raw.to_vec())
}

/// Map from CRLF to LF. Used for block body content where the grammar explicitly
/// allows multi-line text.
pub fn CRLF_TO_LF(text: &str) -> String {
    text.replace("\r\n", "\n")
}

/// Create a bounded debug preview of input text for error reporting.
///
/// Truncates to `DEBUG_PREVIEW_MAX` chars. Replaces unprintable characters
/// with `?` for safe display in diagnostics.
pub fn debug_preview(input: &str) -> String {
    let truncated: String = input
        .chars()
        .take(DEBUG_PREVIEW_MAX)
        .map(|c| {
            if c.is_control() && c != '\n' && c != '\r' && c != '\t' {
                '?'
            } else {
                c
            }
        })
        .collect();

    if input.chars().count() > DEBUG_PREVIEW_MAX {
        format!("{}...", truncated)
    } else {
        truncated
    }
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_nul_bytes() {
        let ctx = ParseContext::default();
        let result = sanitize_control("hello\0world", &ctx);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, DslErrorCode::NulByte);
    }

    #[test]
    fn strips_ansi_sequences() {
        let ctx = ParseContext::default();
        let input = "\u{1b}[31mred text\u{1b}[0m";
        let result = sanitize_control(input, &ctx).unwrap();
        assert_eq!(result, "red text");
    }

    #[test]
    fn strips_osc_sequences() {
        let ctx = ParseContext::default();
        let input = "\u{1b}]0;title\u{07}clean";
        let result = sanitize_control(input, &ctx).unwrap();
        assert_eq!(result, "clean");
    }

    #[test]
    fn passes_clean_text() {
        let ctx = ParseContext::default();
        let input = "STATUS ok\nREASON all good\n";
        let result = sanitize_control(input, &ctx).unwrap();
        assert_eq!(result, "STATUS ok\nREASON all good\n");
    }

    #[test]
    fn crlf_to_lf() {
        assert_eq!(CRLF_TO_LF("line1\r\nline2"), "line1\nline2");
        assert_eq!(CRLF_TO_LF("no change"), "no change");
    }

    #[test]
    fn debug_preview_truncates() {
        let long = "a".repeat(200);
        let preview = debug_preview(&long);
        assert!(preview.len() <= DEBUG_PREVIEW_MAX + 3);
        assert!(preview.ends_with("..."));
    }

    #[test]
    fn debug_preview_short() {
        let short = "hello";
        let preview = debug_preview(short);
        assert_eq!(preview, "hello");
    }

    #[test]
    fn strip_ansi_bytes() {
        let input = b"\x1b[31mred\x1b[0m";
        let out = strip_ansi_for_dsl(input);
        assert_eq!(String::from_utf8_lossy(&out), "red");
    }
}
