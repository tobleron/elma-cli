//! Compact DSL parser primitives and error model.
//!
//! Every structured output an LLM produces is untrusted text. This module provides
//! the shared infrastructure used by all DSL contracts: strict parsing, rejection
//! of malformed input, sanitization, and compact repair feedback.
//!
//! ## Design
//!
//! - DSL family, not one giant language: action DSL, verdict DSLs, scope/list DSLs.
//! - Uppercase command tokens, explicit field names, quoted strings only where needed.
//! - Parser is always authoritative; GBNF grammar is optional defense-in-depth.
//! - No silent fix-ups: quotes, blocks, and terminators must be exact.

mod action;
mod error;
mod parser;
mod render;
mod repair_templates;
mod safety;
mod sanitize;
mod tool_call_xml;

pub use action::{
    classify_action_repair, parse_action_dsl, parse_actions_batch, render_action_repair,
    render_focused_repair, validate_workspace_path, AgentAction,
};
pub use error::{
    ActionRepairKind, DslError, DslErrorCode, DslResult, ParseContext, RepairObservation,
};
pub use parser::{
    expect_command, expect_eol, expect_field_line, expect_key_value, expect_quoted_field,
    expect_terminator, extract_block_body, parse_line, require_field, strip_first_line, DslBlock,
    DslBlockParser, DslField, DslLine,
};
pub use render::{render_compact_error, render_repair_hint, render_repair_hint_with_format};
pub use repair_templates::detect_expected_format;
pub(crate) use safety::{
    apply_exact_edit, ensure_session_edit_snapshot, execute_command_policy,
    execute_command_policy_async, record_session_read, require_session_read_before_edit,
    resolve_workspace_path, validate_command, CommandOutcome, CommandPolicy, ExactEditOutcome,
};
pub use sanitize::{sanitize_control, strip_ansi_for_dsl, CRLF_TO_LF};
pub use tool_call_xml::convert_tool_call_to_action;
pub use tool_call_xml::parse_tool_call_xml;
