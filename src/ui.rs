//! @efficiency-role: orchestrator
//!
//! UI Module (De-bloated)
//!
//! Re-exports from specialized sub-modules:
//! - ui_colors: Gruvbox Dark Hard ANSI color functions
//! - ui_state: Global state management + UI state model
//! - ui_trace: Trace and display functions
//! - ui_chat: Chat HTTP functions
//! - ui_theme: Gruvbox theme constants + ANSI helpers
//! - ui_terminal: TerminalUI — crossterm I/O, event loop
//! - ui_render: Full-screen rendering from UIState
//! - ui_modal: Modal overlay rendering
//! - ui_wrap: ANSI-safe text wrapping

pub(crate) use crate::ui_chat::*;
pub(crate) use crate::ui_modal::render_modal;
pub(crate) use crate::ui_render::render_screen;
pub(crate) use crate::ui_state::*;
pub(crate) use crate::ui_terminal::{MessageRole, TerminalUI};
pub(crate) use crate::ui_theme::*;
pub(crate) use crate::ui_trace::*;
