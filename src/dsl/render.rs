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

fn default_hint_for(code: DslErrorCode) -> &'static str {
    match code {
        DslErrorCode::UnsafePath => "return a safe relative path",
        DslErrorCode::UnsafeCommand => "return a safe command or use another DSL action",
        DslErrorCode::InvalidEdit => "read the file before retrying",
        DslErrorCode::UnsupportedDsl => "use a supported DSL family",
        _ => "return exactly one valid command",
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
            "INVALID_DSL\nerror: missing ---END\nreturn exactly one valid command"
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
}
