//! AgentAction - typed DSL commands for the tool loop.
//!
//! The model outputs exactly one action command per turn using the compact DSL.
//! This module parses and validates those commands into typed Rust `AgentAction`
//! values. Execution is handled by the tool loop.

use crate::dsl::error::{
    ActionRepairKind, DslError, DslErrorCode, DslResult, ParseContext, RepairObservation,
};
use crate::dsl::parser::{
    extract_block_body, parse_header_line, reject_wrapped_output, strip_first_line,
};
use crate::dsl::render::render_compact_error;
use crate::dsl::sanitize::sanitize_control;
use crate::dsl::tool_call_xml::{parse_command_xml, parse_tool_call_json, parse_tool_call_xml};

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
    Next {
        task_id: u32,
        action: String,
        reason: String,
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

    // Try exact provider-style tool markup first. This is a boundary adapter
    // into Rust-native AgentAction, not a general XML extractor.
    if trimmed.contains("<tool_call>") {
        if let Some(action) = parse_tool_call_xml(trimmed) {
            return Ok(vec![action]);
        }
        // Has tool_call markup but parse failed (e.g. mixed prose + markup).
        // Reject deterministically and ask for native action DSL.
        return Err(DslError::invalid_dsl(
            ctx.clone(),
            "mixed prose with <tool_call> markup not allowed; use native action DSL (R, L, S, Y, E, X, ASK, DONE)"
        ));
    }
    if trimmed.contains("<command>") || trimmed.contains("<execute_command>") {
        if let Some(action) = parse_command_xml(trimmed) {
            return Ok(vec![action]);
        }
        // Has command markup but parse failed → mixed prose + markup.
        return Err(DslError::invalid_dsl(
            ctx.clone(),
            "mixed prose with <command> markup not allowed; use native action DSL (R, L, S, Y, E, X, ASK, DONE)"
        ));
    }

    // Try bare JSON tool call format for runtimes that put tool-call-shaped
    // JSON in the assistant content field.
    if trimmed.starts_with('{') {
        if let Some(action) = parse_tool_call_json(trimmed) {
            return Ok(vec![action]);
        }
    }

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
                        | AgentAction::Next { .. }
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

fn parse_action_line(line: &str, remainder: &str, ctx: &ParseContext) -> DslResult<AgentAction> {
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
        "NEXT" => parse_next(&fields, remainder, ctx),
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
    Ok(AgentAction::ReadFile { path })
}

fn parse_list_multi(
    fields: &[crate::dsl::parser::DslField],
    ctx: &ParseContext,
) -> DslResult<AgentAction> {
    ensure_only_fields(fields, &["path", "depth"], ctx)?;
    let path = required_path_field(fields, "path", ctx)?;
    let depth = fields
        .iter()
        .find(|f| f.key == "depth")
        .map(|f| parse_depth(&f.value, ctx))
        .transpose()?
        .unwrap_or(1);
    Ok(AgentAction::ListFiles { path, depth })
}

fn parse_search_text_multi(
    fields: &[crate::dsl::parser::DslField],
    ctx: &ParseContext,
) -> DslResult<AgentAction> {
    ensure_only_fields(fields, &["q", "path"], ctx)?;
    let q = required_non_empty_field(fields, "q", ctx)?.to_string();
    let path = required_path_field(fields, "path", ctx)?;
    Ok(AgentAction::SearchText { q, path })
}

fn parse_search_symbol_multi(
    fields: &[crate::dsl::parser::DslField],
    ctx: &ParseContext,
) -> DslResult<AgentAction> {
    ensure_only_fields(fields, &["q", "path"], ctx)?;
    let q = required_non_empty_field(fields, "q", ctx)?.to_string();
    let path = required_path_field(fields, "path", ctx)?;
    Ok(AgentAction::SearchSymbol { q, path })
}

fn parse_run_command_multi(
    fields: &[crate::dsl::parser::DslField],
    ctx: &ParseContext,
) -> DslResult<AgentAction> {
    ensure_no_fields(fields, ctx)?;
    // Note: X is terminal, so batch parser won't call this in a chain.
    // We still need a version that doesn't require remainder parsing.
    Err(DslError::invalid_dsl(
        ctx.clone(),
        "X requires block body with ---END",
    ))
}

/// Strict single-action parse (original behavior) for error messages
fn parse_action_dsl_single(raw: &str, ctx: &ParseContext) -> DslResult<AgentAction> {
    let sanitized = sanitize_control(raw, ctx)?;
    let trimmed = sanitized.trim();
    reject_wrapped_output(trimmed, ctx)?;
    if trimmed.is_empty() {
        return Err(DslError::empty(ctx.clone()));
    }
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
        "NEXT" => parse_next(&fields, remainder, ctx),
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
    Ok(AgentAction::ReadFile { path })
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
    Ok(AgentAction::ListFiles { path, depth })
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
        path: required_path_field(fields, "path", ctx)?,
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
        path: required_path_field(fields, "path", ctx)?,
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
    Ok(AgentAction::EditFile { path, old, new })
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

/// Parse a NEXT action: pick a task from the pyramid.
/// Format: NEXT task_id=<id> action=<action> reason="..."
fn parse_next(
    fields: &[crate::dsl::parser::DslField],
    remainder: &str,
    ctx: &ParseContext,
) -> DslResult<AgentAction> {
    ensure_no_remainder(remainder, ctx)?;
    ensure_only_fields(fields, &["task_id", "action", "reason"], ctx)?;
    let task_id = fields
        .iter()
        .find(|f| f.key == "task_id")
        .map(|f| {
            f.value.parse::<u32>().map_err(|_| {
                DslError::invalid_field_value(ctx.clone(), format!("task_id: {}", f.value))
            })
        })
        .transpose()?
        .unwrap_or(0);
    let action = fields
        .iter()
        .find(|f| f.key == "action")
        .map(|f| f.value.clone())
        .unwrap_or_default();
    let reason = fields
        .iter()
        .find(|f| f.key == "reason")
        .map(|f| f.value.clone())
        .unwrap_or_default();
    if action.is_empty() {
        return Err(DslError::invalid_dsl(
            ctx.clone(),
            "NEXT requires an action field",
        ));
    }
    Ok(AgentAction::Next {
        task_id,
        action,
        reason,
    })
}

pub(crate) fn parse_next_multi(
    fields: &[crate::dsl::parser::DslField],
    ctx: &ParseContext,
) -> DslResult<AgentAction> {
    ensure_only_fields(fields, &["task_id", "action", "reason"], ctx)?;
    let task_id = fields
        .iter()
        .find(|f| f.key == "task_id")
        .map(|f| {
            f.value.parse::<u32>().map_err(|_| {
                DslError::invalid_field_value(ctx.clone(), format!("task_id: {}", f.value))
            })
        })
        .transpose()?
        .unwrap_or(0);
    let action = fields
        .iter()
        .find(|f| f.key == "action")
        .map(|f| f.value.clone())
        .unwrap_or_default();
    let reason = fields
        .iter()
        .find(|f| f.key == "reason")
        .map(|f| f.value.clone())
        .unwrap_or_default();
    if action.is_empty() {
        return Err(DslError::invalid_dsl(
            ctx.clone(),
            "NEXT requires an action field",
        ));
    }
    Ok(AgentAction::Next {
        task_id,
        action,
        reason,
    })
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

fn required_path_field(
    fields: &[crate::dsl::parser::DslField],
    key: &str,
    ctx: &ParseContext,
) -> DslResult<String> {
    let path = required_non_empty_field(fields, key, ctx)?;
    let normalized = normalize_workspace_path(path);
    validate_workspace_path(&normalized)
        .map_err(|detail| DslError::unsafe_path(ctx.clone(), detail))?;
    Ok(normalized)
}

fn normalize_workspace_path(path: &str) -> String {
    // Normalize common model mistakes before workspace validation:
    // "path=\"/\"" (absolute filesystem root) -> "path=\".\"" (workspace root)
    // Small models naturally reach for "/" to mean "search everywhere".
    // This is non-destructive: any other absolute path still gets rejected.
    let trimmed = path.trim();
    if trimmed == "/" {
        return ".".to_string();
    }

    let candidate = std::path::Path::new(trimmed);
    if candidate.is_absolute() {
        if let Ok(cwd) = std::env::current_dir() {
            if let Ok(relative) = candidate.strip_prefix(&cwd) {
                let relative = relative.to_string_lossy().replace('\\', "/");
                return if relative.trim().is_empty() {
                    ".".to_string()
                } else {
                    relative
                };
            }
        }
    }

    trimmed.to_string()
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
///
/// This is the legacy generic renderer. New code should prefer
/// `classify_action_repair` + `render_focused_repair` for more targeted
/// feedback.
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
        expected_format: Some(
            "R path=\"relative/path\"  |  L path=\"src\" depth=1  |  S q=\"text\" path=\"src\"  |  E path=\"file\"\n---OLD\nold\n---NEW\nnew\n---END  |  X\n<cmd>\n---END  |  DONE summary=\"done\""
                .to_string(),
        ),
    };
    render_compact_error(&observation)
}

/// Classify the *shape* of an action DSL failure from the parse error and the
/// raw model output text. Used by the repair ladder to send focused (rather
/// than generic) feedback.
///
/// Callers pass the pre-parse model output (*before* stripping/trimming) so
/// the classifier can inspect the user-visible text for shape heuristics.
pub fn classify_action_repair(error: &DslError, raw_output: &str) -> ActionRepairKind {
    match error.code {
        DslErrorCode::EmptyOutput => ActionRepairKind::GeneralDsl,
        DslErrorCode::FencedOutput => ActionRepairKind::GeneralDsl,
        DslErrorCode::JsonOutput => ActionRepairKind::BareJson,
        DslErrorCode::XmlOutput => ActionRepairKind::ProviderMarkup,
        DslErrorCode::YamlOutput => ActionRepairKind::GeneralDsl,
        DslErrorCode::TomlOutput => ActionRepairKind::GeneralDsl,
        DslErrorCode::ProseBeforeCommand => ActionRepairKind::ProseBeforeCommand,
        DslErrorCode::ProseAfterCommand => ActionRepairKind::GeneralDsl,
        DslErrorCode::MalformedQuote => ActionRepairKind::MalformedSyntax,
        DslErrorCode::DuplicateField => ActionRepairKind::MalformedSyntax,
        DslErrorCode::MissingField => ActionRepairKind::MissingField,
        DslErrorCode::InvalidFieldValue => ActionRepairKind::MalformedSyntax,
        DslErrorCode::ControlCharacters => ActionRepairKind::MalformedSyntax,
        DslErrorCode::NulByte => ActionRepairKind::MalformedSyntax,
        DslErrorCode::UnknownCommand => ActionRepairKind::UnknownCommand,
        DslErrorCode::InvalidEdit => ActionRepairKind::MissingEditBody,
        DslErrorCode::UnsafePath | DslErrorCode::UnsafeCommand | DslErrorCode::UnsupportedDsl => {
            ActionRepairKind::GeneralDsl
        }
        DslErrorCode::MissingEndMarker => classify_missing_end_marker(raw_output),
        DslErrorCode::InvalidDsl => classify_invalid_dsl(error, raw_output),
    }
}

fn classify_missing_end_marker(raw_output: &str) -> ActionRepairKind {
    let trimmed = raw_output.trim();
    let first_line = trimmed.lines().next().unwrap_or("").trim();
    let first_line_upper = first_line.to_uppercase();
    // Bare X (single token, no fields, no body) → BareCommand
    if first_line_upper == "X" && trimmed.lines().count() <= 1 {
        return ActionRepairKind::BareCommand;
    }
    // E with path= but no body → MissingEditBody
    if first_line_upper.starts_with("E") && first_line.contains("path=") {
        return ActionRepairKind::MissingEditBody;
    }
    ActionRepairKind::MissingEndMarker
}

fn classify_invalid_dsl(error: &DslError, _raw_output: &str) -> ActionRepairKind {
    let detail = error.debug_preview.to_ascii_lowercase();
    // "expected key=value field" → unquoted path
    if detail.contains("expected key=value field") || detail.contains("expected key=value") {
        return ActionRepairKind::UnquotedPath;
    }
    // "mixed prose with <tool_call>" or "<command>" → provider markup
    if detail.contains("<tool_call>") || detail.contains("<command>") {
        return ActionRepairKind::ProviderMarkup;
    }
    ActionRepairKind::GeneralDsl
}

/// Render a focused (or escalated) repair observation for a given repair kind.
///
/// When `escalated` is true the message narrows to a one-bit action-kind
/// decision instead of repeating the same focused hint — this is the "repair
/// ladder" step after three same-family failures.
pub fn render_focused_repair(kind: ActionRepairKind, escalated: bool) -> String {
    let (detail, hint) = focused_repair_text(kind, escalated);
    let code = DslErrorCode::InvalidDsl;
    render_compact_error(&RepairObservation::new(code, detail).with_hint(hint))
}

fn focused_repair_text(kind: ActionRepairKind, escalated: bool) -> (&'static str, &'static str) {
    if escalated {
        return escalated_repair_text(kind);
    }
    match kind {
        ActionRepairKind::BareCommand => (
            "X requires a command body with ---END",
            "use X\n<command>\n---END",
        ),
        ActionRepairKind::UnquotedPath => (
            "unquoted path in action DSL",
            "use R path=\"...\" (quoted path)",
        ),
        ActionRepairKind::MissingEndMarker => (
            "action block is missing ---END",
            "end the block with ---END",
        ),
        ActionRepairKind::MissingEditBody => (
            "edit action missing ---OLD/---NEW sections",
            "use E path=\"...\" with ---OLD and ---NEW markers, then ---END",
        ),
        ActionRepairKind::ProseBeforeCommand => (
            "text found before action DSL command",
            "remove explanatory text, use exactly one action command",
        ),
        ActionRepairKind::ProviderMarkup => (
            "XML/markup wrapper not allowed",
            "use native action DSL: R, L, S, Y, E, X, ASK, DONE",
        ),
        ActionRepairKind::BareJson => (
            "JSON wrapper not allowed",
            "use native action DSL: R, L, S, Y, E, X, ASK, DONE",
        ),
        ActionRepairKind::MissingField => (
            "required field is missing",
            "include required fields (e.g. path=\".\" for workspace root)",
        ),
        ActionRepairKind::MalformedSyntax => (
            "malformed syntax in action DSL",
            "check quote marks, field names, and block markers",
        ),
        ActionRepairKind::UnknownCommand => (
            "unknown action command",
            "use one of: R, L, S, Y, E, X, ASK, DONE",
        ),
        ActionRepairKind::GeneralDsl => (
            "invalid action DSL output",
            "return exactly one valid action command",
        ),
    }
}

fn escalated_repair_text(kind: ActionRepairKind) -> (&'static str, &'static str) {
    match kind {
        ActionRepairKind::BareCommand => (
            "COMMAND action needs a body",
            "use X\n<static command>\n---END or X\n<verification command>\n---END",
        ),
        ActionRepairKind::UnquotedPath => (
            "paths must be quoted",
            "use path=\"relative/path\" not bare text",
        ),
        ActionRepairKind::MissingEndMarker => (
            "all blocks need ---END",
            "end every E, X, ASK, DONE block with ---END",
        ),
        ActionRepairKind::MissingEditBody => (
            "edit needs ---OLD then ---NEW",
            "use E path=\"...\" with ---OLD / ---NEW / ---END",
        ),
        ActionRepairKind::ProseBeforeCommand => (
            "only one action command, no explanation",
            "start directly with R, L, S, Y, E, X, ASK, or DONE",
        ),
        ActionRepairKind::ProviderMarkup => (
            "no XML or function calls",
            "use native DSL only: R, L, S, Y, E, X, ASK, DONE",
        ),
        ActionRepairKind::BareJson => (
            "no JSON wrappers",
            "use native DSL only: R, L, S, Y, E, X, ASK, DONE",
        ),
        ActionRepairKind::MissingField => (
            "every action needs its fields",
            "R needs path=, S needs q= and path=, E needs path=",
        ),
        ActionRepairKind::MalformedSyntax => (
            "syntax problem",
            "use key=\"value\" syntax with matching quotes",
        ),
        ActionRepairKind::UnknownCommand => (
            "unknown token",
            "choose only from: R, L, S, Y, E, X, ASK, DONE",
        ),
        ActionRepairKind::GeneralDsl => (
            "invalid action DSL",
            "return exactly one valid action command",
        ),
    }
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
    fn test_batch_parses_tool_call_xml() {
        let actions = parse_actions_batch(
            r#"<tool_call>{"name":"read","arguments":{"path":"Cargo.toml"}}</tool_call>"#,
            &ctx(),
            3,
        )
        .unwrap();
        assert_eq!(
            actions,
            vec![AgentAction::ReadFile {
                path: "Cargo.toml".to_string()
            }]
        );
    }

    #[test]
    fn test_batch_parses_command_xml_as_shell() {
        let actions =
            parse_actions_batch("<command>ls -la AGENTS.md</command>", &ctx(), 3).unwrap();
        assert_eq!(
            actions,
            vec![AgentAction::RunCommand {
                command: "ls -la AGENTS.md".to_string()
            }]
        );
    }

    #[test]
    fn test_batch_parses_command_xml_wrapping_native_action() {
        let actions =
            parse_actions_batch("<command>\nR path=\"AGENTS.md\"\n</command>", &ctx(), 3).unwrap();
        assert_eq!(
            actions,
            vec![AgentAction::ReadFile {
                path: "AGENTS.md".to_string()
            }]
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
    fn test_parse_workspace_absolute_path_as_relative() {
        let cwd = std::env::current_dir().unwrap();
        let path = cwd.join("src/main.rs");
        let raw = format!(r#"R path="{}""#, path.display());
        let action = parse_action_dsl(&raw, &ctx()).unwrap();
        assert!(matches!(action, AgentAction::ReadFile { path } if path == "src/main.rs"));
    }

    #[test]
    fn test_render_action_repair_hint() {
        let err = DslError::missing_end_marker(ctx(), "missing ---END".to_string());
        let rendered = render_action_repair(&err);
        assert!(rendered.contains("INVALID_DSL"));
        assert!(rendered.contains("---END"));
    }

    // ── ActionRepairKind classification tests ──

    #[test]
    fn test_classify_bare_command() {
        let err = DslError::missing_end_marker(ctx(), "missing ---END".to_string());
        assert_eq!(
            classify_action_repair(&err, "X"),
            ActionRepairKind::BareCommand
        );
        let err = DslError::missing_end_marker(ctx(), "missing ---END".to_string());
        assert_eq!(
            classify_action_repair(&err, "  X  "),
            ActionRepairKind::BareCommand
        );
    }

    #[test]
    fn test_classify_unquoted_path() {
        let err = DslError::invalid_dsl(ctx(), "expected key=value field, got src/main.rs");
        assert_eq!(
            classify_action_repair(&err, "R src/main.rs"),
            ActionRepairKind::UnquotedPath
        );
    }

    #[test]
    fn test_classify_missing_edit_body() {
        let err = DslError::missing_end_marker(ctx(), "missing ---END".to_string());
        assert_eq!(
            classify_action_repair(&err, "E path=\"foo\""),
            ActionRepairKind::MissingEditBody
        );
    }

    #[test]
    fn test_classify_bare_json() {
        let err = DslError::json_output(ctx(), "json output".to_string());
        assert_eq!(
            classify_action_repair(&err, "{\"name\":\"shell\",\"args\":{}}"),
            ActionRepairKind::BareJson
        );
    }

    #[test]
    fn test_classify_unknown_command() {
        let err = DslError::unknown_command(ctx(), "Z");
        assert_eq!(
            classify_action_repair(&err, "Z path=\"foo\""),
            ActionRepairKind::UnknownCommand
        );
    }

    #[test]
    fn test_classify_prose_before() {
        let err = DslError::prose_before(ctx(), "let me look".to_string());
        assert_eq!(
            classify_action_repair(&err, "let me look\nR path=\"x\""),
            ActionRepairKind::ProseBeforeCommand
        );
    }

    #[test]
    fn test_classify_missing_field() {
        let err = DslError::missing_field(ctx(), "path");
        assert_eq!(
            classify_action_repair(&err, "R"),
            ActionRepairKind::MissingField
        );
    }

    #[test]
    fn test_classify_missing_end_marker_generic() {
        let err = DslError::missing_end_marker(ctx(), "missing ---END".to_string());
        assert_eq!(
            classify_action_repair(&err, "ASK\nwhat is this"),
            ActionRepairKind::MissingEndMarker
        );
    }

    // ── render_focused_repair tests ──

    #[test]
    fn test_focused_repair_bare_command_normal() {
        let repair = render_focused_repair(ActionRepairKind::BareCommand, false);
        assert!(repair.contains("command body"));
        assert!(repair.contains("X"));
        assert!(repair.contains("---END"));
    }

    #[test]
    fn test_focused_repair_bare_command_escalated() {
        let repair = render_focused_repair(ActionRepairKind::BareCommand, true);
        assert!(repair.contains("COMMAND action needs a body"));
        assert!(repair.contains("X"));
    }

    #[test]
    fn test_focused_repair_unquoted_path_normal() {
        let repair = render_focused_repair(ActionRepairKind::UnquotedPath, false);
        assert!(repair.contains("unquoted path"));
        assert!(repair.contains("path=\"...\""));
    }

    #[test]
    fn test_focused_repair_unquoted_path_escalated() {
        let repair = render_focused_repair(ActionRepairKind::UnquotedPath, true);
        assert!(repair.contains("paths must be quoted"));
        assert!(repair.contains("path="));
    }

    #[test]
    fn test_focused_repair_missing_edit_body() {
        let repair = render_focused_repair(ActionRepairKind::MissingEditBody, false);
        assert!(repair.contains("---OLD"));
        assert!(repair.contains("---NEW"));
    }

    #[test]
    fn test_focused_repair_provider_markup() {
        let repair = render_focused_repair(ActionRepairKind::ProviderMarkup, false);
        assert!(repair.contains("XML"));
        assert!(repair.contains("R, L, S, Y, E, X, ASK, DONE"));
    }

    #[test]
    fn test_focused_repair_general_escalated_still_sensible() {
        // GeneralDsl escalated should still produce a reasonable message
        let repair = render_focused_repair(ActionRepairKind::GeneralDsl, true);
        assert!(repair.contains("action DSL"));
        assert!(!repair.is_empty());
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

    #[test]
    fn test_batch_rejects_mixed_prose_with_tool_call() {
        let err = parse_actions_batch(
            "I'll read the file\n<tool_call>{\"name\":\"read\",\"arguments\":{\"path\":\"Cargo.toml\"}}</tool_call>",
            &ctx(),
            3,
        )
        .unwrap_err();
        assert_eq!(err.code, DslErrorCode::InvalidDsl);
        assert!(err.to_string().contains("tool_call"));
    }

    #[test]
    fn test_batch_rejects_mixed_prose_with_command() {
        let err = parse_actions_batch("Let me run this\n<command>ls -la</command>", &ctx(), 3)
            .unwrap_err();
        assert_eq!(err.code, DslErrorCode::InvalidDsl);
        assert!(err.to_string().contains("command"));
    }

    #[test]
    fn test_batch_rejects_text_after_tool_call() {
        let err = parse_actions_batch(
            "<tool_call>{\"name\":\"read\",\"arguments\":{\"path\":\"Cargo.toml\"}}</tool_call>\nmore text",
            &ctx(),
            3,
        )
        .unwrap_err();
        assert_eq!(err.code, DslErrorCode::InvalidDsl);
    }

    #[test]
    fn test_batch_rejects_text_before_tool_call() {
        let err = parse_actions_batch(
            "text before\n<tool_call>{\"name\":\"shell\",\"arguments\":{\"command\":\"ls\"}}</tool_call>",
            &ctx(),
            3,
        )
        .unwrap_err();
        assert_eq!(err.code, DslErrorCode::InvalidDsl);
    }
}
