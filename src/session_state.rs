//! @efficiency-role: data-model
//! Session-scoped state container.
//!
//! Consolidates all mutable session state into a single struct that is
//! passed through the call chain instead of accessed via global statics.
//! This is the migration target for Tasks 554+.

use crate::command_budget::CommandBudget;
use crate::evidence_ledger::EvidenceLedger;
use crate::safe_mode::SafeMode;
use std::collections::HashSet;

/// All mutable state for a single agent session.
/// Passed via `&mut SessionState` through the execution chain.
pub struct SessionState {
    pub evidence_ledger: Option<EvidenceLedger>,
    pub safe_mode: SafeMode,
    pub command_budget: CommandBudget,
    pub confirmed_commands: HashSet<String>,
    pub event_log_turn_id: Option<String>,
}

impl SessionState {
    pub fn new() -> Self {
        Self {
            evidence_ledger: None,
            safe_mode: SafeMode::Ask,
            command_budget: CommandBudget::new(),
            confirmed_commands: HashSet::new(),
            event_log_turn_id: None,
        }
    }
}
