//! @efficiency-role: service-orchestrator
//!
//! UI Module (De-bloated)
//!
//! Re-exports from specialized sub-modules:
//! - ui_colors: ANSI color functions
//! - ui_state: State management
//! - ui_trace: Trace and display functions
//! - ui_chat: Chat functions

pub use crate::ui_chat::*;
pub use crate::ui_colors::*;
pub use crate::ui_state::*;
pub use crate::ui_trace::*;
