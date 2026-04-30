//! AgentAction - typed DSL commands for the tool loop.
//!
//! The model outputs exactly one action command per turn using the compact DSL.
//! This module parses and validates those commands into typed Rust `AgentAction`
//! values. Execution is handled by the tool loop.

use crate::dsl::error::{DslError, DslErrorCode, DslResult, ParseContext, RepairObservation};
use crate::dsl::parser::{
    extract_block_body, parse_header_line, reject_wrapped_output, strip_first_line,
};
use crate::dsl::render::render_compact_error;
use crate::dsl::sanitize::sanitize_control;

/// One typed, validated action parsed from model DSL output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentAction {
    ReadFile {
        path: String,
    },
    ListFiles {
        path: String,
        depth: u8,
    },
    SearchText {
        q: String,
        path: String,
    },
    SearchSymbol {
        q: String,
        path: String,
    },
    EditFile {
        path: String,
        old: String,
        new: String,
    },
    RunCommand {
        command: String,
    },
    Ask {
        question: String,
    },
    Done {
        summary: String,
    },
}

/// Parse a complete model turn into one `AgentAction`.
pub fn parse_action_dsl(raw: &str, ctx: &ParseContext) -> DslResult<AgentAction> {
    parse_action_dsl_single(raw, ctx)
}

/// Parse up to `max` AgentActions from consecutive DSL command lines.
/// Read-only actions (R, L, S, Y) can be batched. Write or terminal
/// actions (E, X, ASK, DONE) stop the batch after the first action.
/// Returns at least one action on success, or an error if no valid
/// commands were found.
const MAX_BATCH_ACTIONS: usize = 3;

pub fn parse_actions_batch(
    raw: &str,
    ctx: &ParseContext,
    max: usize,
) -> DslResult<Vec<AgentAction>> {
    let sanitized = sanitize_control(raw, ctx)?;
    let trimmed = sanitized.trim();
    reject_wrapped_output(trimmed, ctx)?;
    if trimmed.is_empty() {
        return Err(DslError::empty(ctx.clone()));
    }

    let limit = max.min(MAX_BATCH_ACTIONS);
    let mut remaining = trimmed;
    let mut actions = Vec::new();

    while actions.len() < limit && !remaining.is_empty() {
        // Try to skip blank lines between commands
        let non_blank = remaining.trim_start();
        if non_blank.is_empty() {
            break;
        }

        let (line, rest) = strip_first_line(non_blank);
        let line_ctx = ctx.with_line(actions.len() + 1);

        match parse_action_line(line, rest, &line_ctx) {
            Ok(action) => {
                let is_terminal = matches!(
                    action,
                    AgentAction::Ask { .. }
                        | AgentAction::Done { .. }
                        | AgentAction::EditFile { .. }
                        | AgentAction::RunCommand { .. }
                );
                actions.push(action);
                if is_terminal {
                    break;
                }
                remaining = rest;
            }
            Err(_) => {
                // If we haven't parsed anything yet, fail.
                // If we already have actions, stop gracefully.
                if actions.is_empty() {
                    // Re-parse with strict single-action to get proper error
                    return parse_action_dsl_single(trimmed, ctx).map(|a| vec![a]);
                }
                break;
            }
        }
    }

    if actions.is_empty() {
        return parse_action_dsl_single(trimmed, ctx).map(|a| vec![a]);
    }
    Ok(actions)
}

fn parse_action_line(
    line: &str,
    remainder: &str,
    ctx: &ParseContext,
) -> DslResult<AgentAction> {
    let (command, fields) = parse_header_line(line, ctx)?;
    match command.as_str() {
        "R" => parse_read_multi(&fields, ctx),
        "L" => parse_list_multi(&fields, ctx),
        "S" => parse_search_text_multi(&fields, ctx),
        "Y" => parse_search_symbol_multi(&fields, ctx),
        "E" => parse_edit(&fields, remainder, ctx),
        "X" => parse_run_command(&fields, remainder, ctx),
        "ASK" => parse_ask(&fields, remainder, ctx),
        "DONE" => parse_done(&fields, remainder, ctx),
        _ => Err(DslError::unknown_command(ctx.clone(), &command)),
    }
}

// Multi-action versions: parse only header fields, ignore remainder.
// Used by parse_actions_batch to chain consecutive commands.
// The remainder is handled by the batch parser itself.

fn parse_read_multi(
    fields: &[crate::dsl::parser::DslField],
    ctx: &ParseContext,
) -> DslResult<AgentAction> {
    ensure_only_fields(fields, &["path"], ctx)?;
    let path = required_path_field(fields, "path", ctx)?;
    Ok(AgentAction::ReadFile { path: path.to_string() })
}

fn parse_list_multi(
    fields: &[crate::dsl::parser::DslField],
    ctx: &ParseContext,
) -> DslResult<AgentAction> {
    ensure_only_fields(fields, &["path", "depth"], ctx)?;
    let path = required_path_field(fields, "path", ctx)?;
    let depth = fields.iter().find(|f| f.key == "depth")
        .map(|f| parse_depth(&f.value, ctx)).transpose()?.unwrap_or(1);
    Ok(AgentAction::ListFiles { path: path.to_string(), depth })
}

fn parse_search_text_multi(
    fields: &[crate::dsl::parser::DslField],
    ctx: &ParseContext,
) -> DslResult<AgentAction> {
    ensure_only_fields(fields, &["q", "path"], ctx)?;
    let q = required_non_empty_field(fields, "q", ctx)?.to_string();
    let path = required_path_field(fields, "path", ctx)?.to_string();
    Ok(AgentAction::SearchText { q, path })
}

fn parse_search_symbol_multi(
    fields: &[crate::dsl::parser::DslField],
    ctx: &ParseContext,
) -> DslResult<AgentAction> {
    ensure_only_fields(fields, &["q", "path"], ctx)?;
    let q = required_non_empty_field(fields, "q", ctx)?.to_string();
    let path = required_path_field(fields, "path", ctx)?.to_string();
    Ok(AgentAction::SearchSymbol { q, path })
}

fn parse_run_command_multi(
    fields: &[crate::dsl::parser::DslField],
    ctx: &ParseContext,
) -> DslResult<AgentAction> {
    ensure_no_fields(fields, ctx)?;
    // Note: X is terminal, so batch parser won't call this in a chain.
    // We still need a version that doesn't require remainder parsing.
    Err(DslError::invalid_dsl(ctx.clone(), "X requires block body with ---END"))
}

/// Strict single-action parse (original behavior) for error messages
fn parse_action_dsl_single(raw: &str, ctx: &ParseContext) -> DslResult<AgentAction> {
    let sanitized = sanitize_control(raw, ctx)?;
    let trimmed = sanitized.trim();
    reject_wrapped_output(trimmed, ctx)?;
    if trimmed.is_empty() { return Err(DslError::empty(ctx.clone())); }
    let (first_line, remainder) = strip_first_line(trimmed);
    let (command, fields) = parse_header_line(first_line, ctx)?;
    match command.as_str() {
        "R" => parse_read(&fields, remainder, ctx),
        "L" => parse_list(&fields, remainder, ctx),
        "S" => parse_search_text(&fields, remainder, ctx),
        "Y" => parse_search_symbol(&fields, remainder, ctx),
        "E" => parse_edit(&fields, remainder, ctx),
        "X" => parse_run_command(&fields, remainder, ctx),
        "ASK" => parse_ask(&fields, remainder, ctx),
        "DONE" => parse_done(&fields, remainder, ctx),
        _ => Err(DslError::unknown_command(ctx.clone(), &command)),
    }
}

fn parse_read(
    fields: &[crate::dsl::parser::DslField],
    remainder: &str,
    ctx: &ParseContext,
) -> DslResult<AgentAction> {
    ensure_no_remainder(remainder, ctx)?;
    ensure_only_fields(fields, &["path"], ctx)?;
    let path = required_path_field(fields, "path", ctx)?;
    Ok(AgentAction::ReadFile {
        path: path.to_string(),
    })
}

fn parse_list(
    fields: &[crate::dsl::parser::DslField],
    remainder: &str,
    ctx: &ParseContext,
) -> DslResult<AgentAction> {
    ensure_no_remainder(remainder, ctx)?;
    ensure_only_fields(fields, &["path", "depth"], ctx)?;
    let path = required_path_field(fields, "path", ctx)?;
    let depth = fields
        .iter()
        .find(|field| field.key == "depth")
        .map(|field| parse_depth(&field.value, ctx))
        .transpose()?
        .unwrap_or(1);
    Ok(AgentAction::ListFiles {
        path: path.to_string(),
        depth,
    })
}

fn parse_search_text(
    fields: &[crate::dsl::parser::DslField],
    remainder: &str,
    ctx: &ParseContext,
) -> DslResult<AgentAction> {
    ensure_no_remainder(remainder, ctx)?;
    ensure_only_fields(fields, &["q", "path"], ctx)?;
    Ok(AgentAction::SearchText {
        q: required_non_empty_field(fields, "q", ctx)?.to_string(),
        path: required_path_field(fields, "path", ctx)?.to_string(),
    })
}

fn parse_search_symbol(
    fields: &[crate::dsl::parser::DslField],
    remainder: &str,
    ctx: &ParseContext,
) -> DslResult<AgentAction> {
    ensure_no_remainder(remainder, ctx)?;
    ensure_only_fields(fields, &["q", "path"], ctx)?;
    Ok(AgentAction::SearchSymbol {
        q: required_non_empty_field(fields, "q", ctx)?.to_string(),
        path: required_path_field(fields, "path", ctx)?.to_string(),
    })
}

fn parse_edit(
    fields: &[crate::dsl::parser::DslField],
    remainder: &str,
    ctx: &ParseContext,
) -> DslResult<AgentAction> {
    ensure_only_fields(fields, &["path"], ctx)?;
    let path = required_path_field(fields, "path", ctx)?;
    let (body, trailing) = extract_block_body(remainder, "---END", ctx)?;
    if !trailing.trim().is_empty() {
        return Err(DslError::prose_after(
            ctx.clone(),
            trailing.trim().to_string(),
        ));
    }
    let (old, new) = extract_edit_sections(&body, ctx)?;
    Ok(AgentAction::EditFile {
        path: path.to_string(),
        old,
        new,
    })
}

fn parse_run_command(
    fields: &[crate::dsl::parser::DslField],
    remainder: &str,
    ctx: &ParseContext,
) -> DslResult<AgentAction> {
    ensure_no_fields(fields, ctx)?;
    let (body, trailing) = extract_block_body(remainder, "---END", ctx)?;
    if !trailing.trim().is_empty() {
        return Err(DslError::prose_after(
            ctx.clone(),
            trailing.trim().to_string(),
        ));
    }
    let command = body.trim().to_string();
    if command.is_empty() {
        return Err(DslError::empty(ctx.clone()));
    }
    Ok(AgentAction::RunCommand { command })
}

fn parse_ask(
    fields: &[crate::dsl::parser::DslField],
    remainder: &str,
    ctx: &ParseContext,
) -> DslResult<AgentAction> {
    ensure_no_fields(fields, ctx)?;
    let (body, trailing) = extract_block_body(remainder, "---END", ctx)?;
    if !trailing.trim().is_empty() {
        return Err(DslError::prose_after(
            ctx.clone(),
            trailing.trim().to_string(),
        ));
    }
    let question = body.trim().to_string();
    if question.is_empty() {
        return Err(DslError::empty(ctx.clone()));
    }
    Ok(AgentAction::Ask { question })
}

fn parse_done(
    fields: &[crate::dsl::parser::DslField],
    remainder: &str,
    ctx: &ParseContext,
) -> DslResult<AgentAction> {
    // Accept single-line DONE with summary field:
    //   DONE summary="compact one-line summary"
    if !fields.is_empty() {
        if fields.len() == 1 && fields[0].key == "summary" {
            let summary = fields[0].value.trim().to_string();
            if summary.is_empty() {
                return Err(DslError::empty(ctx.clone()));
            }
            ensure_no_remainder(remainder, ctx)?;
            return Ok(AgentAction::Done { summary });
        }
        return Err(DslError::invalid_dsl(
            ctx.clone(),
            "unexpected header fields for DONE; use DONE summary=\"...\" for single-line or DONE\\n...\\n---END for multi-line",
        ));
    }
    // Accept block DONE with ---END terminator:
    //   DONE
    //   multi-line summary text
    //   ---END
    let (body, trailing) = extract_block_body(remainder, "---END", ctx)?;
    if !trailing.trim().is_empty() {
        return Err(DslError::prose_after(
            ctx.clone(),
            trailing.trim().to_string(),
        ));
    }
    let summary = body.trim().to_string();
    if summary.is_empty() {
        return Err(DslError::empty(ctx.clone()));
    }
    Ok(AgentAction::Done { summary })
}

fn extract_edit_sections(body: &str, ctx: &ParseContext) -> DslResult<(String, String)> {
    let mut phase = 0usize;
    let mut old_lines = Vec::new();
    let mut new_lines = Vec::new();

    for line in body.lines() {
        let trimmed = line.trim();
        match trimmed {
            "---OLD" if phase == 0 => {
                phase = 1;
                continue;
            }
            "---NEW" if phase == 1 => {
                phase = 2;
                continue;
            }
            "---OLD" | "---NEW" => {
                return Err(DslError::invalid_edit(
                    ctx.clone(),
                    format!("unexpected {trimmed} marker"),
                ));
            }
            _ => {}
        }

        match phase {
            0 => {
                if !trimmed.is_empty() {
                    return Err(DslError::invalid_edit(
                        ctx.clone(),
                        "edit body must start with ---OLD",
                    ));
                }
            }
            1 => old_lines.push(line.to_string()),
            2 => new_lines.push(line.to_string()),
            _ => {}
        }
    }

    if phase < 2 {
        return Err(DslError::missing_end_marker(
            ctx.clone(),
            "missing ---NEW or ---END".to_string(),
        ));
    }

    let old = old_lines.join("\n");
    let new = new_lines.join("\n");
    if old.trim().is_empty() || new.trim().is_empty() {
        return Err(DslError::invalid_edit(
            ctx.clone(),
            "edit sections must both be non-empty",
        ));
    }

    Ok((old, new))
}

fn parse_depth(value: &str, ctx: &ParseContext) -> DslResult<u8> {
    let depth = value.trim().parse::<u8>().map_err(|_| {
        DslError::invalid_field_value(ctx.clone(), format!("invalid depth: {value}"))
    })?;
    if !(1..=5).contains(&depth) {
        return Err(DslError::invalid_field_value(
            ctx.clone(),
            format!("depth out of range: {depth}"),
        ));
    }
    Ok(depth)
}

fn ensure_no_remainder(remainder: &str, ctx: &ParseContext) -> DslResult<()> {
    if remainder.trim().is_empty() {
        Ok(())
    } else {
        Err(DslError::prose_after(
            ctx.clone(),
            remainder.trim().to_string(),
        ))
    }
}

fn required_field<'a>(
    fields: &'a [crate::dsl::parser::DslField],
    key: &str,
    ctx: &ParseContext,
) -> DslResult<&'a str> {
    crate::dsl::parser::require_field(fields, key, ctx)
}

fn required_non_empty_field<'a>(
    fields: &'a [crate::dsl::parser::DslField],
    key: &str,
    ctx: &ParseContext,
) -> DslResult<&'a str> {
    let value = required_field(fields, key, ctx)?;
    if value.trim().is_empty() {
        return Err(DslError::invalid_field_value(
            ctx.clone(),
            format!("field {key} must not be empty"),
        ));
    }
    Ok(value)
}

fn required_path_field<'a>(
    fields: &'a [crate::dsl::parser::DslField],
    key: &str,
    ctx: &ParseContext,
) -> DslResult<&'a str> {
    let path = required_non_empty_field(fields, key, ctx)?;
    // Normalize common model mistakes before workspace validation:
    // "path=\"/\"" (absolute filesystem root) -> "path=\".\"" (workspace root)
    // Small models naturally reach for "/" to mean "search everywhere".
    // This is non-destructive: any other absolute path still gets rejected.
    let normalized = if path.trim() == "/" { "." } else { path };
    validate_workspace_path(normalized).map_err(|detail| DslError::unsafe_path(ctx.clone(), detail))?;
    Ok(normalized)
}

fn ensure_no_fields(fields: &[crate::dsl::parser::DslField], ctx: &ParseContext) -> DslResult<()> {
    if fields.is_empty() {
        Ok(())
    } else {
        Err(DslError::invalid_dsl(
            ctx.clone(),
            "unexpected header fields",
        ))
    }
}

fn ensure_only_fields(
    fields: &[crate::dsl::parser::DslField],
    allowed: &[&str],
    ctx: &ParseContext,
) -> DslResult<()> {
    for field in fields {
        if !allowed.iter().any(|allowed_key| *allowed_key == field.key) {
            return Err(DslError::invalid_dsl(
                ctx.clone(),
                format!("unexpected field: {}", field.key),
            ));
        }
    }
    Ok(())
}

/// Validate a workspace-relative path is safe.
pub fn validate_workspace_path(path: &str) -> Result<(), String> {
    if path.starts_with('/') {
        return Err("absolute path not allowed; use project-relative path".to_string());
    }
    if path.is_empty() {
        return Err("path must not be empty".to_string());
    }
    if path.split('/').any(|part| part == "..") || path.contains("/../") || path.ends_with("/..") {
        return Err("path escapes project root via ..".to_string());
    }
    Ok(())
}

/// Render a compact repair observation from a DSL parse error.
pub fn render_action_repair(error: &DslError) -> String {
    let observation = RepairObservation {
        code: error.code,
        detail: format!(
            "action DSL parse error: {}",
            if error.debug_preview.is_empty() {
                error.to_string()
            } else {
                error.debug_preview.clone()
            }
        ),
        hint: Some(match error.code {
            DslErrorCode::UnknownCommand => "use one of: R, L, S, Y, E, X, ASK, DONE".to_string(),
            DslErrorCode::MissingField => "include required field (e.g. path=, q=)".to_string(),
            DslErrorCode::MissingEndMarker => "end block with ---END".to_string(),
            DslErrorCode::InvalidEdit => "use ---OLD / ---NEW / ---END markers".to_string(),
            DslErrorCode::EmptyOutput => "return exactly one action command".to_string(),
            DslErrorCode::InvalidFieldValue => {
                "use non-empty field values; use path=\".\" for workspace root".to_string()
            }
            DslErrorCode::UnsafePath => {
                "use a safe workspace-relative path; use path=\".\" for workspace root".to_string()
            }
            _ => "return exactly one valid action command".to_string(),
        }),
    };
    render_compact_error(&observation)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> ParseContext {
        ParseContext {
            dsl_variant: "action",
            line: None,
        }
    }

    #[test]
    fn test_parse_read() {
        let action = parse_action_dsl("R path=\"src/main.rs\"", &ctx()).unwrap();
        assert_eq!(
            action,
            AgentAction::ReadFile {
                path: "src/main.rs".to_string()
            }
        );
    }

    #[test]
    fn test_parse_list_defaults_depth() {
        let action = parse_action_dsl("L path=\"src\"", &ctx()).unwrap();
        assert_eq!(
            action,
            AgentAction::ListFiles {
                path: "src".to_string(),
                depth: 1,
            }
        );
    }

    #[test]
    fn test_parse_search_text() {
        let action = parse_action_dsl("S q=\"needle\" path=\"src\"", &ctx()).unwrap();
        assert_eq!(
            action,
            AgentAction::SearchText {
                q: "needle".to_string(),
                path: "src".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_search_symbol() {
        let action = parse_action_dsl("Y q=\"symbol_name\" path=\"src\"", &ctx()).unwrap();
        assert_eq!(
            action,
            AgentAction::SearchSymbol {
                q: "symbol_name".to_string(),
                path: "src".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_edit() {
        let action = parse_action_dsl(
            "E path=\"src/main.rs\"\n---OLD\nold text\n---NEW\nnew text\n---END",
            &ctx(),
        )
        .unwrap();
        assert_eq!(
            action,
            AgentAction::EditFile {
                path: "src/main.rs".to_string(),
                old: "old text".to_string(),
                new: "new text".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_ask() {
        let action = parse_action_dsl("ASK\nWhat is your goal?\n---END", &ctx()).unwrap();
        assert_eq!(
            action,
            AgentAction::Ask {
                question: "What is your goal?".to_string()
            }
        );
    }

    #[test]
    fn test_parse_done() {
        let action = parse_action_dsl("DONE\nsummary text\n---END", &ctx()).unwrap();
        assert_eq!(
            action,
            AgentAction::Done {
                summary: "summary text".to_string()
            }
        );
    }

    #[test]
    fn test_parse_run_command() {
        let action = parse_action_dsl("X\ncargo test\n---END", &ctx()).unwrap();
        assert_eq!(
            action,
            AgentAction::RunCommand {
                command: "cargo test".to_string()
            }
        );
    }

    #[test]
    fn test_rejects_fenced_output() {
        let err = parse_action_dsl("```\nDONE\nok\n---END\n```", &ctx()).unwrap_err();
        assert_eq!(err.code, DslErrorCode::FencedOutput);
    }

    #[test]
    fn test_rejects_json_output() {
        let err = parse_action_dsl("{\"action\":\"done\"}", &ctx()).unwrap_err();
        assert_eq!(err.code, DslErrorCode::JsonOutput);
    }

    #[test]
    fn test_rejects_empty() {
        let err = parse_action_dsl("", &ctx()).unwrap_err();
        assert_eq!(err.code, DslErrorCode::EmptyOutput);
    }

    #[test]
    fn test_rejects_unknown_command() {
        let err = parse_action_dsl("Z path=\"foo\"", &ctx()).unwrap_err();
        assert_eq!(err.code, DslErrorCode::UnknownCommand);
    }

    #[test]
    fn test_rejects_prose_after_command() {
        let err = parse_action_dsl("R path=\"src/main.rs\"\nmore text", &ctx()).unwrap_err();
        assert_eq!(err.code, DslErrorCode::ProseAfterCommand);
    }

    #[test]
    fn test_rejects_missing_end_marker() {
        let err = parse_action_dsl("DONE\nsummary text", &ctx()).unwrap_err();
        assert_eq!(err.code, DslErrorCode::MissingEndMarker);
    }

    #[test]
    fn test_rejects_duplicate_fields() {
        let err = parse_action_dsl("R path=\"a\" path=\"b\"", &ctx()).unwrap_err();
        assert_eq!(err.code, DslErrorCode::DuplicateField);
    }

    #[test]
    fn test_rejects_malformed_quotes() {
        let err = parse_action_dsl("R path=\"src/main.rs", &ctx()).unwrap_err();
        assert_eq!(err.code, DslErrorCode::MalformedQuote);
    }

    #[test]
    fn test_validate_workspace_path() {
        assert!(validate_workspace_path("src/main.rs").is_ok());
        assert!(validate_workspace_path("/etc/passwd").is_err());
        assert!(validate_workspace_path("../escape").is_err());
        assert!(validate_workspace_path("").is_err());
    }

    #[test]
    fn test_render_action_repair_hint() {
        let err = DslError::missing_end_marker(ctx(), "missing ---END".to_string());
        let rendered = render_action_repair(&err);
        assert!(rendered.contains("INVALID_DSL"));
        assert!(rendered.contains("---END"));
    }

    #[test]
    fn test_depth_bounds() {
        assert!(matches!(
            parse_action_dsl("L path=\"src\" depth=5", &ctx()).unwrap(),
            AgentAction::ListFiles { depth: 5, .. }
        ));
        let err = parse_action_dsl("L path=\"src\" depth=0", &ctx()).unwrap_err();
        assert_eq!(err.code, DslErrorCode::InvalidFieldValue);
    }

    #[test]
    fn test_parse_read_missing_path() {
        let err = parse_action_dsl("R", &ctx()).unwrap_err();
        assert_eq!(err.code, DslErrorCode::MissingField);
    }

    #[test]
    fn test_parse_list_missing_path() {
        let err = parse_action_dsl("L depth=3", &ctx()).unwrap_err();
        assert_eq!(err.code, DslErrorCode::MissingField);
    }

    #[test]
    fn test_rejects_empty_path_field() {
        let err = parse_action_dsl("R path=\"\"", &ctx()).unwrap_err();
        assert_eq!(err.code, DslErrorCode::InvalidFieldValue);
    }

    #[test]
    fn test_rejects_unsafe_action_path() {
        let err = parse_action_dsl("L path=\"../outside\" depth=2", &ctx()).unwrap_err();
        assert_eq!(err.code, DslErrorCode::UnsafePath);
    }

    #[test]
    fn test_rejects_empty_search_query() {
        let err = parse_action_dsl("S q=\"\" path=\"src\"", &ctx()).unwrap_err();
        assert_eq!(err.code, DslErrorCode::InvalidFieldValue);
    }

    #[test]
    fn test_parse_search_missing_fields() {
        let err1 = parse_action_dsl("S q=\"needle\"", &ctx()).unwrap_err();
        assert_eq!(err1.code, DslErrorCode::MissingField);
        let err2 = parse_action_dsl("S path=\"src\"", &ctx()).unwrap_err();
        assert_eq!(err2.code, DslErrorCode::MissingField);
    }

    #[test]
    fn test_parse_edit_missing_fields() {
        let err = parse_action_dsl("E path=\"file.txt\"", &ctx()).unwrap_err();
        assert_eq!(err.code, DslErrorCode::MissingEndMarker);
        let err2 = parse_action_dsl("E path=\"file.txt\"\n---OLD\nold\n---NEW\nnew\n", &ctx())
            .unwrap_err();
        assert_eq!(err2.code, DslErrorCode::MissingEndMarker);
    }

    #[test]
    fn test_rejects_unknown_field() {
        let err = parse_action_dsl("R path=\"a\" unknown=\"x\"", &ctx()).unwrap_err();
        assert_eq!(err.code, DslErrorCode::InvalidDsl);
    }

    #[test]
    fn test_ask_empty_body() {
        let err = parse_action_dsl("ASK\n---END", &ctx()).unwrap_err();
        assert_eq!(err.code, DslErrorCode::EmptyOutput);
    }

    #[test]
    fn test_done_empty_body() {
        let err = parse_action_dsl("DONE\n---END", &ctx()).unwrap_err();
        assert_eq!(err.code, DslErrorCode::EmptyOutput);
    }

    #[test]
    fn test_run_command_empty_body() {
        let err = parse_action_dsl("X\n---END", &ctx()).unwrap_err();
        assert_eq!(err.code, DslErrorCode::EmptyOutput);
    }

    #[test]
    fn test_ask_rejects_unknown_field() {
        let err = parse_action_dsl("ASK extra=\"x\"\nquestion\n---END", &ctx()).unwrap_err();
        assert_eq!(err.code, DslErrorCode::InvalidDsl);
    }

    #[test]
    fn test_done_rejects_unknown_field() {
        let err = parse_action_dsl("DONE extra=\"x\"\nsummary\n---END", &ctx()).unwrap_err();
        assert_eq!(err.code, DslErrorCode::InvalidDsl);
    }

    #[test]
    fn test_run_command_rejects_unknown_field() {
        let err = parse_action_dsl("X extra=\"x\"\ncmd\n---END", &ctx()).unwrap_err();
        assert_eq!(err.code, DslErrorCode::InvalidDsl);
    }

    #[test]
    fn test_rejects_unknown_field_for_edit() {
        let err = parse_action_dsl(
            "E path=\"a\" unknown=\"x\"\n---OLD\no\n---NEW\nn\n---END",
            &ctx(),
        )
        .unwrap_err();
        assert_eq!(err.code, DslErrorCode::InvalidDsl);
    }

    #[test]
    fn test_rejects_unknown_field_for_search() {
        let err =
            parse_action_dsl("S q=\"needle\" path=\"src\" unknown=\"x\"", &ctx()).unwrap_err();
        assert_eq!(err.code, DslErrorCode::InvalidDsl);
    }

    #[test]
    fn test_rejects_unknown_field_for_list() {
        let err = parse_action_dsl("L path=\"src\" depth=2 unknown=\"x\"", &ctx()).unwrap_err();
        assert_eq!(err.code, DslErrorCode::InvalidDsl);
    }

    #[test]
    fn test_rejects_missing_path_for_list() {
        let err = parse_action_dsl("L", &ctx()).unwrap_err();
        assert_eq!(err.code, DslErrorCode::MissingField);
    }

    #[test]
    fn test_rejects_list_depth_zero() {
        let err = parse_action_dsl("L path=\"src\" depth=0", &ctx()).unwrap_err();
        assert_eq!(err.code, DslErrorCode::InvalidFieldValue);
    }

    #[test]
    fn test_rejects_list_depth_too_high() {
        let err = parse_action_dsl("L path=\"src\" depth=10", &ctx()).unwrap_err();
        assert_eq!(err.code, DslErrorCode::InvalidFieldValue);
    }
}
