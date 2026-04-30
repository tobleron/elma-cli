//! DSL error types: strict, deterministic, non-panicking rejection for untrusted model output.
//!
//! Every parser error carries a stable error code suitable for compact repair
//! observations. The error type also stores a bounded debug preview of the rejected
//! input so diagnostics don't leak full model output.

use std::fmt;

/// Stable error codes used by all DSL parsers.
///
/// New codes should be added here when new DSL contracts require new rejection
/// semantics.  Codes are intentionally uppercase and short for compact rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DslErrorCode {
    /// Model returned nothing (empty string or whitespace-only).
    EmptyOutput,

    /// Input starts with ``` or is wrapped in a Markdown code fence.
    FencedOutput,

    /// Input looks like JSON ({ or [).
    JsonOutput,

    /// Input looks like XML (<).
    XmlOutput,

    /// Input looks like YAML (--- or key: value without DSL prefix).
    YamlOutput,

    /// Input looks like TOML ([[section]] or [section]).
    TomlOutput,

    /// Prose (explanatory text) found before the first DSL command.
    ProseBeforeCommand,

    /// Prose found after the DSL command/block terminator.
    ProseAfterCommand,

    /// A required quote character is missing (unterminated).
    MalformedQuote,

    /// A block end marker (e.g. `---END`) is missing.
    MissingEndMarker,

    /// A field appears more than once in a single record.
    DuplicateField,

    /// A required field is missing from the record.
    MissingField,

    /// A field value is not valid (wrong type, empty when required).
    InvalidFieldValue,

    /// Unknown command token (not in the expected set).
    UnknownCommand,

    /// Input contains NUL bytes.
    NulByte,

    /// Input contains ANSI escape sequences or unprintable control chars.
    ControlCharacters,

    /// General malformed DSL - not covering any of the above.
    InvalidDsl,

    /// Path is unsafe (contains `..`, absolute, symlink escape).
    UnsafePath,

    /// Command is not permitted by current policy.
    UnsafeCommand,

    /// Edit specification is invalid (missing anchor, duplicate).
    InvalidEdit,

    /// DSL variant is not supported by this parser.
    UnsupportedDsl,
}

impl fmt::Display for DslErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            DslErrorCode::EmptyOutput => "INVALID_DSL",
            DslErrorCode::FencedOutput => "INVALID_DSL",
            DslErrorCode::JsonOutput => "INVALID_DSL",
            DslErrorCode::XmlOutput => "INVALID_DSL",
            DslErrorCode::YamlOutput => "INVALID_DSL",
            DslErrorCode::TomlOutput => "INVALID_DSL",
            DslErrorCode::ProseBeforeCommand => "INVALID_DSL",
            DslErrorCode::ProseAfterCommand => "INVALID_DSL",
            DslErrorCode::MalformedQuote => "INVALID_DSL",
            DslErrorCode::MissingEndMarker => "INVALID_DSL",
            DslErrorCode::DuplicateField => "INVALID_DSL",
            DslErrorCode::MissingField => "INVALID_DSL",
            DslErrorCode::InvalidFieldValue => "INVALID_DSL",
            DslErrorCode::UnknownCommand => "INVALID_DSL",
            DslErrorCode::NulByte => "INVALID_DSL",
            DslErrorCode::ControlCharacters => "INVALID_DSL",
            DslErrorCode::InvalidDsl => "INVALID_DSL",
            DslErrorCode::UnsafePath => "UNSAFE_PATH",
            DslErrorCode::UnsafeCommand => "UNSAFE_COMMAND",
            DslErrorCode::InvalidEdit => "INVALID_EDIT",
            DslErrorCode::UnsupportedDsl => "UNSUPPORTED_DSL",
        };
        write!(f, "{}", s)
    }
}

/// Parse context carried through DSL parsing for diagnostics.
#[derive(Debug, Clone)]
pub struct ParseContext {
    /// Name of the DSL variant being parsed (e.g. "verdict", "action").
    pub dsl_variant: &'static str,

    /// Line number (1-based) where the error occurred, if known.
    pub line: Option<usize>,
}

impl Default for ParseContext {
    fn default() -> Self {
        Self {
            dsl_variant: "dsl",
            line: None,
        }
    }
}

impl ParseContext {
    pub fn with_line(&self, line: usize) -> Self {
        let mut next = self.clone();
        next.line = Some(line);
        next
    }
}

/// A DSL parse error: code + bounded debug preview.
#[derive(Debug, Clone)]
pub struct DslError {
    pub code: DslErrorCode,
    pub context: ParseContext,

    /// Truncated preview of the offending input (max ~80 chars).
    pub debug_preview: String,
}

impl fmt::Display for DslError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(line) = self.context.line {
            write!(
                f,
                "{} ({}:{}): {}",
                self.code, self.context.dsl_variant, line, self.debug_preview
            )
        } else {
            write!(
                f,
                "{} ({}): {}",
                self.code, self.context.dsl_variant, self.debug_preview
            )
        }
    }
}

impl std::error::Error for DslError {}

/// Alias for Result with DslError.
pub type DslResult<T> = Result<T, DslError>;

/// Compact repair observation sent as feedback to the model on parse failure.
///
/// This is NOT a log message — it is the text the model receives to prompt a
/// corrected response. It must be short, deterministic, and free of verbose
/// suggestions.
#[derive(Debug, Clone)]
pub struct RepairObservation {
    /// The error code as uppercase token.
    pub code: DslErrorCode,

    /// One short sentence describing what was wrong.
    pub detail: String,

    /// Optional single-line instruction for correction.
    pub hint: Option<String>,
}

impl RepairObservation {
    pub fn new(code: DslErrorCode, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
            hint: None,
        }
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }
}

// ── Helper constructors for common errors ──

impl DslError {
    pub fn new(code: DslErrorCode, context: ParseContext, debug_preview: String) -> Self {
        Self {
            code,
            context,
            debug_preview,
        }
    }

    pub fn empty(context: ParseContext) -> Self {
        Self::new(DslErrorCode::EmptyOutput, context, String::new())
    }

    pub fn fenced(context: ParseContext, preview: String) -> Self {
        Self::new(DslErrorCode::FencedOutput, context, preview)
    }

    pub fn json_output(context: ParseContext, preview: String) -> Self {
        Self::new(DslErrorCode::JsonOutput, context, preview)
    }

    pub fn prose_before(context: ParseContext, preview: String) -> Self {
        Self::new(DslErrorCode::ProseBeforeCommand, context, preview)
    }

    pub fn prose_after(context: ParseContext, preview: String) -> Self {
        Self::new(DslErrorCode::ProseAfterCommand, context, preview)
    }

    pub fn malformed_quote(context: ParseContext, preview: String) -> Self {
        Self::new(DslErrorCode::MalformedQuote, context, preview)
    }

    pub fn missing_end_marker(context: ParseContext, preview: String) -> Self {
        Self::new(DslErrorCode::MissingEndMarker, context, preview)
    }

    pub fn duplicate_field(context: ParseContext, field: &str) -> Self {
        Self::new(
            DslErrorCode::DuplicateField,
            context,
            format!("duplicate field: {}", field),
        )
    }

    pub fn missing_field(context: ParseContext, field: &str) -> Self {
        Self::new(
            DslErrorCode::MissingField,
            context,
            format!("missing field: {}", field),
        )
    }

    pub fn invalid_field_value(context: ParseContext, value: impl Into<String>) -> Self {
        Self::new(DslErrorCode::InvalidFieldValue, context, value.into())
    }

    pub fn unknown_command(context: ParseContext, command: &str) -> Self {
        Self::new(
            DslErrorCode::UnknownCommand,
            context,
            format!("unknown command: {}", command),
        )
    }

    pub fn invalid_dsl(context: ParseContext, detail: impl Into<String>) -> Self {
        Self::new(DslErrorCode::InvalidDsl, context, detail.into())
    }

    pub fn unsafe_path(context: ParseContext, path: impl Into<String>) -> Self {
        Self::new(
            DslErrorCode::UnsafePath,
            context,
            format!("path escapes project root: {}", path.into()),
        )
    }

    pub fn unsafe_command(context: ParseContext, command: impl Into<String>) -> Self {
        Self::new(
            DslErrorCode::UnsafeCommand,
            context,
            format!("command is not allowed by policy: {}", command.into()),
        )
    }

    pub fn invalid_edit(context: ParseContext, detail: impl Into<String>) -> Self {
        Self::new(DslErrorCode::InvalidEdit, context, detail.into())
    }

    pub fn unsupported_dsl(context: ParseContext, detail: impl Into<String>) -> Self {
        Self::new(DslErrorCode::UnsupportedDsl, context, detail.into())
    }
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_are_stable() {
        assert_eq!(DslErrorCode::EmptyOutput.to_string(), "INVALID_DSL");
        assert_eq!(DslErrorCode::FencedOutput.to_string(), "INVALID_DSL");
        assert_eq!(DslErrorCode::JsonOutput.to_string(), "INVALID_DSL");
        assert_eq!(DslErrorCode::MissingEndMarker.to_string(), "INVALID_DSL");
        assert_eq!(DslErrorCode::DuplicateField.to_string(), "INVALID_DSL");
        assert_eq!(DslErrorCode::MalformedQuote.to_string(), "INVALID_DSL");
        assert_eq!(DslErrorCode::UnsafePath.to_string(), "UNSAFE_PATH");
        assert_eq!(DslErrorCode::UnsafeCommand.to_string(), "UNSAFE_COMMAND");
        assert_eq!(DslErrorCode::InvalidEdit.to_string(), "INVALID_EDIT");
        assert_eq!(DslErrorCode::UnsupportedDsl.to_string(), "UNSUPPORTED_DSL");
    }

    #[test]
    fn empty_dsl_error() {
        let err = DslError::empty(ParseContext::default());
        assert_eq!(err.code, DslErrorCode::EmptyOutput);
    }

    #[test]
    fn repair_observation_format() {
        let obs = RepairObservation::new(DslErrorCode::MissingEndMarker, "missing ---END")
            .with_hint("return exactly one complete block");
        assert_eq!(obs.code, DslErrorCode::MissingEndMarker);
        assert_eq!(obs.detail, "missing ---END");
        assert_eq!(
            obs.hint,
            Some("return exactly one complete block".to_string())
        );
    }
}
