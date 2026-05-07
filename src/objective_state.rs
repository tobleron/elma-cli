//! @efficiency-role: domain-logic
//!
//! Objective State and Approach Supervisor — Task 763.
//!
//! Tracks the current turn's objective, required outcomes, active approach,
//! and blockers. Wraps direct tool-calling with approach-aware supervision:
//! when repeated failures occur on a branch, the supervisor forks a sibling
//! approach with a changed strategy rather than continuing down the failing path.

use crate::*;
use std::path::Path;

/// Unique identifier for a supervision cycle within a turn.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct SupervisionId(pub(crate) String);

impl SupervisionId {
    pub fn new() -> Self {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        Self(format!("sup_{}_{}", ts.as_secs(), ts.subsec_nanos()))
    }
}

impl Default for SupervisionId {
    fn default() -> Self {
        Self::new()
    }
}

/// The kind of outcome the user expects from this turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum OutcomeKind {
    /// The user wants an answer to a question.
    Answer,
    /// The user wants one or more files created or modified.
    Deliverable,
    /// The user wants exploration or investigation.
    Exploration,
    /// The user wants verification of existing state.
    Verification,
}

/// A single required outcome for the current turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RequiredOutcome {
    pub label: String,
    pub kind: OutcomeKind,
    pub target: Option<String>,
    pub completed: bool,
    pub evidence_ids: Vec<String>,
}

/// Task 763: Per-turn objective state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ObjectiveState {
    /// The raw user objective for this turn.
    pub raw_objective: String,
    /// Required outcomes extracted from the objective.
    pub required_outcomes: Vec<RequiredOutcome>,
    /// The currently active approach identifier.
    pub active_approach_id: String,
    /// Approach statuses: approach_id -> status
    pub approaches: std::collections::HashMap<String, String>,
    /// Evidence IDs that have been collected.
    pub completed_evidence: Vec<String>,
    /// Current blockers that prevent progress.
    pub blockers: Vec<String>,
    /// Requirements that remain unresolved.
    pub unresolved_requirements: Vec<String>,
    /// How many times the current approach has stalled.
    pub stagnation_count: u32,
    /// Total approach attempts for this turn.
    pub total_approaches: u32,
}

impl ObjectiveState {
    pub fn new(raw_objective: &str) -> Self {
        let approach_id = format!("a_init_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos());
        let mut approaches = std::collections::HashMap::new();
        approaches.insert(approach_id.clone(), "active".to_string());
        Self {
            raw_objective: raw_objective.to_string(),
            required_outcomes: Vec::new(),
            active_approach_id: approach_id,
            approaches,
            completed_evidence: Vec::new(),
            blockers: Vec::new(),
            unresolved_requirements: Vec::new(),
            stagnation_count: 0,
            total_approaches: 1,
        }
    }

    /// Fork a new sibling approach. Marks the current one as pruned.
    pub fn fork_approach(&mut self, reason: &str) -> String {
        let new_id = format!("a_fork_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos());
        self.approaches
            .insert(self.active_approach_id.clone(), format!("pruned: {}", reason));
        self.approaches.insert(new_id.clone(), "active".to_string());
        self.active_approach_id = new_id.clone();
        self.total_approaches += 1;
        self.stagnation_count = 0;
        new_id
    }

    /// Record evidence that was collected.
    pub fn record_evidence(&mut self, evidence_id: &str) {
        if !self.completed_evidence.contains(&evidence_id.to_string()) {
            self.completed_evidence.push(evidence_id.to_string());
        }
    }

    /// Add a blocker.
    pub fn add_blocker(&mut self, blocker: &str) {
        if !self.blockers.contains(&blocker.to_string()) {
            self.blockers.push(blocker.to_string());
        }
    }

    /// Mark all outcomes as completed (for finalization).
    pub fn mark_all_completed(&mut self) {
        for outcome in &mut self.required_outcomes {
            outcome.completed = true;
        }
    }

    /// Whether all required outcomes are completed.
    /// Returns true if there are no required outcomes (vacuous truth).
    pub fn all_outcomes_completed(&self) -> bool {
        self.required_outcomes.is_empty()
            || self.required_outcomes.iter().all(|o| o.completed)
    }

    /// Whether any outcome remains unresolved.
    pub fn has_unresolved(&self) -> bool {
        self.required_outcomes
            .iter()
            .any(|o| !o.completed)
            || !self.unresolved_requirements.is_empty()
    }

    /// Whether the state indicates a need to fork to a new approach.
    pub fn needs_approach_fork(&self, max_stagnation: u32) -> bool {
        self.stagnation_count >= max_stagnation
    }

    /// Persist the objective state to session storage.
    pub fn persist(&self, session_root: &Path) {
        let dir = session_root.join("objective_state");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("state.json");
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(&path, &json);
        }
    }
}

// ── tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_objective_state_new() {
        let state = ObjectiveState::new("find AGENTS.md");
        assert_eq!(state.raw_objective, "find AGENTS.md");
        assert_eq!(state.total_approaches, 1);
        assert!(state.required_outcomes.is_empty());
        assert!(state.blockers.is_empty());
    }

    #[test]
    fn test_objective_state_fork_approach() {
        let mut state = ObjectiveState::new("test");
        let original = state.active_approach_id.clone();
        let new_id = state.fork_approach("stagnation");
        assert_ne!(new_id, original);
        assert_eq!(state.active_approach_id, new_id);
        assert_eq!(state.total_approaches, 2);
        assert_eq!(state.stagnation_count, 0);
        assert_eq!(state.approaches.len(), 2);
    }

    #[test]
    fn test_objective_state_record_evidence() {
        let mut state = ObjectiveState::new("test");
        state.record_evidence("e_001");
        assert_eq!(state.completed_evidence.len(), 1);
        // Duplicate should be ignored
        state.record_evidence("e_001");
        assert_eq!(state.completed_evidence.len(), 1);
    }

    #[test]
    fn test_objective_state_blockers() {
        let mut state = ObjectiveState::new("test");
        state.add_blocker("path not found: tasks/completed");
        assert_eq!(state.blockers.len(), 1);
        // Duplicate should be ignored
        state.add_blocker("path not found: tasks/completed");
        assert_eq!(state.blockers.len(), 1);
    }

    #[test]
    fn test_objective_state_all_outcomes_completed() {
        let mut state = ObjectiveState::new("test");
        // No outcomes means all completed vacuously
        assert!(state.all_outcomes_completed());
        state.required_outcomes.push(RequiredOutcome {
            label: "read file".to_string(),
            kind: OutcomeKind::Exploration,
            target: Some("AGENTS.md".to_string()),
            completed: false,
            evidence_ids: Vec::new(),
        });
        assert!(!state.all_outcomes_completed());
        state.mark_all_completed();
        assert!(state.all_outcomes_completed());
    }

    #[test]
    fn test_objective_state_needs_approach_fork() {
        let mut state = ObjectiveState::new("test");
        assert!(!state.needs_approach_fork(3));
        state.stagnation_count = 3;
        assert!(state.needs_approach_fork(3));
    }

    #[test]
    fn test_objective_state_has_unresolved() {
        let mut state = ObjectiveState::new("test");
        assert!(!state.has_unresolved());
        state.unresolved_requirements.push("need path".to_string());
        assert!(state.has_unresolved());
    }

    #[test]
    fn test_objective_state_persist() {
        let tmp = std::env::temp_dir().join("objective_state_test");
        let _ = std::fs::create_dir_all(&tmp);
        let state = ObjectiveState::new("test objective");
        state.persist(&tmp);
        let path = tmp.join("objective_state").join("state.json");
        assert!(path.exists());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_supervision_id_unique() {
        let id1 = SupervisionId::new();
        let id2 = SupervisionId::new();
        assert_ne!(id1, id2);
    }
}
