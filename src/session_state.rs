//! @efficiency-role: domain-logic
//! Session State (Task 782)
//!
//! Centralized repository for mutable session-scoped state.
//! Consolidates previously scattered OnceLock globals to improve
//! testability, observability, and dataflow transparency.

use std::sync::{Arc, Mutex, RwLock};
use std::collections::{HashMap, HashSet};
use crate::safe_mode::SafeMode;
use crate::workspace_policy::ScopeConstraint;
use crate::event_log::EventLog;
use crate::evidence_ledger::EvidenceLedger;
use crate::artifact_verifier::{DeliverableContract, ArtifactManifest};
use crate::permission_gate::ApprovalCache;
use crate::command_budget::CommandBudget;

pub(crate) struct SafetySettings {
    pub shell_redirection_blocked: bool,
    pub path_escape_blocked: bool,
    pub max_shell_calls_per_turn: usize,
}

impl Default for SafetySettings {
    fn default() -> Self {
        Self {
            shell_redirection_blocked: true,
            path_escape_blocked: true,
            max_shell_calls_per_turn: 10,
        }
    }
}

pub(crate) struct SessionState {
    pub safe_mode: Mutex<SafeMode>,
    pub safety_settings: Mutex<SafetySettings>,
    pub has_mutated: RwLock<bool>,
    pub scope_constraint: RwLock<Option<ScopeConstraint>>,
    pub network_disabled: RwLock<bool>,
    pub no_color: RwLock<bool>,
    pub event_log: RwLock<Option<EventLog>>,
    pub current_turn_id: RwLock<Option<String>>,
    pub evidence_ledger: RwLock<Option<EvidenceLedger>>,
    pub deliverable_contract: RwLock<Option<DeliverableContract>>,
    pub required_artifacts: RwLock<HashSet<String>>,
    pub artifact_manifest: RwLock<ArtifactManifest>,
    pub permission_cache: Mutex<ApprovalCache>,
    pub budget: Mutex<CommandBudget>,
}

impl SessionState {
    pub fn new() -> Self {
        Self {
            safe_mode: Mutex::new(SafeMode::default()),
            safety_settings: Mutex::new(SafetySettings::default()),
            has_mutated: RwLock::new(false),
            scope_constraint: RwLock::new(None),
            network_disabled: RwLock::new(false),
            no_color: RwLock::new(false),
            event_log: RwLock::new(None),
            current_turn_id: RwLock::new(None),
            evidence_ledger: RwLock::new(None),
            deliverable_contract: RwLock::new(None),
            required_artifacts: RwLock::new(HashSet::new()),
            artifact_manifest: RwLock::new(ArtifactManifest::default()),
            permission_cache: Mutex::new(ApprovalCache::default()),
            budget: Mutex::new(CommandBudget::new()),
        }
    }
}

static SESSION_STATE: std::sync::OnceLock<Arc<SessionState>> = std::sync::OnceLock::new();

pub(crate) fn get_session_state() -> Arc<SessionState> {
    SESSION_STATE.get_or_init(|| Arc::new(SessionState::new())).clone()
}

/// For testing: reset the global session state.
#[cfg(test)]
pub(crate) fn reset_session_state() {
    // Note: OnceLock cannot be reset, but we can provide a way to clear the contents
    // if we wrap it differently. For now, we'll focus on consolidation.
}
