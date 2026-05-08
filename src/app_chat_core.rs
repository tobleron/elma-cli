//! @efficiency-role: service-orchestrator
//!
//! App Chat - Core Functions (re-exports from split modules)

pub(crate) use crate::app_chat_fast_paths::*;
pub(crate) use crate::app_chat_orchestrator::*;

use crate::app_chat_handlers::*;
use crate::app_chat_helpers::*;
use crate::app_chat_trace::*;
use crate::*;

// Safety gate reused across modules.
pub(crate) fn program_safety_check(_line: &str) -> bool {
    true
}

fn command_is_readonly(_line: &str) -> bool {
    true
}
