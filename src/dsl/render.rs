//! Compact repair rendering for DSL parse and validation failures.
//!
//! The renderer keeps feedback short enough for constrained local models to
//! recover without needing a verbose explanation layer.

use crate::dsl::error::{DslError, DslErrorCode, RepairObservation};

pub fn render_compact_error(observation: &RepairObservation) -> String {
    let mut out = String::new();
    out.push_str(&observation.code.to_string());
    out.push('\n');
    out.push_str("error: ");
    out.push_str(observation.detail.trim());
    out.push('\n');
    if let Some(ref expected_format) = observation.expected_format {
        out.push_str("Expected: ");
        out.push_str(expected_format.trim());
        out.push('\n');
    }
    if let Some(hint) = observation.hint.as_deref() {
        out.push_str(hint.trim());
    } else {
        out.push_str(default_hint_for(observation.code));
    }
    out
}

pub fn render_repair_hint(error: &DslError) -> String {
    let observation = RepairObservation::new(error.code, error.debug_preview.clone());
    render_compact_error(&observation)
}

pub fn render_repair_hint_with_format(error: &DslError, expected_format: &str) -> String {
    let observation = RepairObservation::new(error.code, error.debug_preview.clone())
        .with_expected_format(expected_format);
    render_compact_error(&observation)
}

fn default_hint_for(code: DslErrorCode) -> &'static str {
    match code {
        DslErrorCode::EmptyOutput => "return exactly one DSL line, not empty content",
        DslErrorCode::FencedOutput => "remove Markdown ``` fences, return raw DSL",
        DslErrorCode::JsonOutput => "do not wrap output in JSON { }, use native DSL format",
        DslErrorCode::XmlOutput => "do not wrap output in XML tags, use native DSL format",
        DslErrorCode::ProseBeforeCommand => "do not include reasoning text before the DSL command",
        DslErrorCode::ProseAfterCommand => "do not include reasoning text after the DSL command",
        DslErrorCode::MalformedQuote => {
            "check that all quoted strings have matching opening and closing quotes"
        }
        DslErrorCode::MissingEndMarker => "close the block with ---END",
        DslErrorCode::DuplicateField => "each field must appear only once per DSL line",
        DslErrorCode::MissingField => "include all required fields for this DSL command",
        DslErrorCode::InvalidFieldValue => {
            "use allowed values for each field (see Expected format above)"
        }
        DslErrorCode::UnknownCommand => "use a valid uppercase DSL command token",
        DslErrorCode::UnsafePath => "return a safe relative path",
        DslErrorCode::UnsafeCommand => "return a safe command or use another DSL action",
        DslErrorCode::InvalidEdit => "read the file before retrying",
        DslErrorCode::UnsupportedDsl => "use a supported DSL family",
        DslErrorCode::YamlOutput => "do not wrap output in YAML format, use native DSL",
        DslErrorCode::TomlOutput => "do not wrap output in TOML format, use native DSL",
        DslErrorCode::NulByte => "remove NUL bytes from output",
        DslErrorCode::ControlCharacters => "remove ANSI escape codes and control characters",
        DslErrorCode::InvalidDsl => "return exactly one valid DSL command",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::error::{DslError, ParseContext};

    #[test]
    fn agent_protocol_renders_missing_end_marker() {
        let obs = RepairObservation::new(DslErrorCode::MissingEndMarker, "missing ---END");
        let rendered = render_compact_error(&obs);
        assert_eq!(
            rendered,
            "INVALID_DSL\nerror: missing ---END\nclose the block with ---END"
        );
    }

    #[test]
    fn agent_protocol_renders_unsafe_path_hint() {
        let obs = RepairObservation::new(DslErrorCode::UnsafePath, "path escapes project root");
        let rendered = render_compact_error(&obs);
        assert_eq!(
            rendered,
            "UNSAFE_PATH\nerror: path escapes project root\nreturn a safe relative path"
        );
    }

    #[test]
    fn render_repair_hint_uses_error_preview() {
        let err =
            DslError::missing_end_marker(ParseContext::default(), "missing ---END".to_string());
        let rendered = render_repair_hint(&err);
        assert!(rendered.contains("INVALID_DSL"));
        assert!(rendered.contains("missing ---END"));
    }

    #[test]
    fn repair_hint_with_expected_format() {
        let err = DslError::unknown_command(ParseContext::default(), "BADCMD");
        let rendered = render_repair_hint_with_format(
            &err,
            "MODE choice=N label=LABEL reason=\"justification\" entropy=N.N",
        );
        assert!(rendered.contains("INVALID_DSL"));
        assert!(rendered.contains("unknown command: BADCMD"));
        assert!(rendered.contains("Expected: MODE choice=N label=LABEL"));
        assert!(rendered.contains("use a valid uppercase DSL command token"));
    }

    #[test]
    fn repair_observation_with_expected_format() {
        let obs = RepairObservation::new(DslErrorCode::MissingField, "missing field: path")
            .with_expected_format("R path=\"relative/path\"");
        let rendered = render_compact_error(&obs);
        assert!(rendered.contains("Expected: R path=\"relative/path\""));
        assert!(rendered.contains("include all required fields"));
    }
}
