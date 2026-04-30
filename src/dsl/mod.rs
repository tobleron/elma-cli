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
mod safety;
mod sanitize;

pub use action::{parse_action_dsl, parse_actions_batch, render_action_repair, validate_workspace_path, AgentAction};
pub use error::{DslError, DslErrorCode, DslResult, ParseContext, RepairObservation};
pub use parser::{
    expect_command, expect_eol, expect_field_line, expect_key_value, expect_quoted_field,
    expect_terminator, extract_block_body, parse_line, require_field, strip_first_line, DslBlock,
    DslBlockParser, DslField, DslLine,
};
pub use render::{render_compact_error, render_repair_hint};
pub(crate) use safety::{
    apply_exact_edit, ensure_session_edit_snapshot, execute_command_policy, record_session_read,
    require_session_read_before_edit, resolve_workspace_path, CommandOutcome, CommandPolicy,
    ExactEditOutcome,
};
pub use sanitize::{sanitize_control, strip_ansi_for_dsl, CRLF_TO_LF};
