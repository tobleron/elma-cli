//! @efficiency-role: orchestrator
//!
//! App Chat Module (De-bloated)
//!
//! Re-exports from specialized sub-modules:
//! - app_chat_core: Core chat loop and program building
//! - app_chat_handlers: Command handlers (/snapshot, /rollback, /tune, etc.)
//! - app_chat_trace: Trace functions for classification/planning
//! - app_chat_helpers: Helper functions (workspace refresh, memory saving)

pub(crate) use crate::app_chat_core::*;
pub(crate) use crate::app_chat_handlers::*;
pub(crate) use crate::app_chat_helpers::*;
pub(crate) use crate::app_chat_trace::*;
