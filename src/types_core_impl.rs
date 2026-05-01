//! @efficiency-role: data-model
//!
//! Implementation blocks for core types (split from types_core.rs)

use crate::types_core::{
    CalibrationMetric, CalibrationReport, CalibrationSummary, CandidateScore, GoalState,
    ParameterBands, SearchCandidate, Step,
};

impl GoalState {
    pub fn new(objective: String) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            active_objective: Some(objective),
            completed_subgoals: Vec::new(),
            pending_subgoals: Vec::new(),
            blocked_reason: None,
            created_at: now,
            last_updated: now,
        }
    }

    pub fn complete_subgoal(&mut self, subgoal: String) {
        self.pending_subgoals.retain(|p| p != &subgoal);
        if !self.completed_subgoals.contains(&subgoal) {
            self.completed_subgoals.push(subgoal);
        }
        self.last_updated = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    pub fn add_pending_subgoal(&mut self, subgoal: String) {
        if !self.pending_subgoals.contains(&subgoal) && !self.completed_subgoals.contains(&subgoal)
        {
            self.pending_subgoals.push(subgoal);
            self.last_updated = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
        }
    }

    pub fn clear(&mut self) {
        self.active_objective = None;
        self.completed_subgoals.clear();
        self.pending_subgoals.clear();
        self.blocked_reason = None;
    }

    pub fn has_active_goal(&self) -> bool {
        self.active_objective.is_some() && self.blocked_reason.is_none()
    }
}

impl Default for CalibrationMetric {
    fn default() -> Self {
        Self {
            total: 0,
            correct: 0,
            accuracy: 0.0,
        }
    }
}

impl Default for CalibrationSummary {
    fn default() -> Self {
        Self {
            total_cases: 0,
            speech_act: CalibrationMetric::default(),
            workflow: CalibrationMetric::default(),
            mode: CalibrationMetric::default(),
            route: CalibrationMetric::default(),
            program_parse: CalibrationMetric::default(),
            program_shape: CalibrationMetric::default(),
            program_policy: CalibrationMetric::default(),
            program_consistency: CalibrationMetric::default(),
            execution: CalibrationMetric::default(),
            critic: CalibrationMetric::default(),
            response: CalibrationMetric::default(),
            scope: CalibrationMetric::default(),
            compaction: CalibrationMetric::default(),
            classification: CalibrationMetric::default(),
            claim_check: CalibrationMetric::default(),
            presentation: CalibrationMetric::default(),
            all_ok: CalibrationMetric::default(),
            certified: false,
            certification_rule: String::new(),
        }
    }
}

impl Default for CalibrationReport {
    fn default() -> Self {
        Self {
            version: 1,
            model: String::new(),
            base_url: String::new(),
            supports_logprobs: false,
            n_probs: 64,
            summary: CalibrationSummary::default(),
            speech_act_confusions: Vec::new(),
            workflow_confusions: Vec::new(),
            mode_confusions: Vec::new(),
            route_confusions: Vec::new(),
            scenarios: Vec::new(),
        }
    }
}

impl Default for CandidateScore {
    fn default() -> Self {
        Self {
            name: String::new(),
            dir: std::path::PathBuf::new(),
            report: crate::types_core::CalibrationReport::default(),
            score: 0.0,
            hard_rejected: false,
            variance: 0.0,
            std_dev: 0.0,
            parse_failure_count: 0,
            latency_avg_ms: 0.0,
        }
    }
}

impl Default for SearchCandidate {
    fn default() -> Self {
        Self {
            name: String::new(),
            dir: std::path::PathBuf::new(),
            score: 0.0,
            hard_rejected: false,
            variance: 0.0,
            std_dev: 0.0,
        }
    }
}

impl ParameterBands {
    /// Get safe parameter bands for a given unit type
    pub fn for_unit_type(unit_type: &str) -> Self {
        match unit_type {
            // Routing/verification/JSON units: near-deterministic
            "speech_act"
            | "router"
            | "mode_router"
            | "critic"
            | "logical_reviewer"
            | "efficiency_reviewer"
            | "risk_reviewer"
            | "outcome_verifier"
            | "execution_sufficiency"
            | "command_preflight"
            | "task_semantics_guard" => Self {
                temperature: (0.0, 0.1),
                top_p: (1.0, 1.0),
                repeat_penalty: (1.0, 1.0),
                max_tokens: (64, 256),
            },
            // Orchestration units: low creativity
            "orchestrator"
            | "workflow_planner"
            | "formula_selector"
            | "scope_builder"
            | "complexity_assessor"
            | "refinement" => Self {
                temperature: (0.2, 0.5),
                top_p: (0.9, 1.0),
                repeat_penalty: (1.0, 1.1),
                max_tokens: (2048, 4096),
            },
            // Response units: modest creativity
            "elma"
            | "summarizer"
            | "expert_advisor"
            | "result_presenter"
            | "formatter"
            | "claim_checker"
            | "evidence_mode"
            | "evidence_compactor"
            | "artifact_classifier" => Self {
                temperature: (0.4, 0.7),
                top_p: (0.9, 1.0),
                repeat_penalty: (1.0, 1.2),
                max_tokens: (1024, 4096),
            },
            // Default bands
            _ => Self {
                temperature: (0.2, 0.6),
                top_p: (0.9, 1.0),
                repeat_penalty: (1.0, 1.1),
                max_tokens: (1024, 4096),
            },
        }
    }
}

impl Step {
    pub(crate) fn id(&self) -> &str {
        match self {
            Step::Shell { id, .. }
            | Step::Read { id, .. }
            | Step::Observe { id, .. }
            | Step::Search { id, .. }
            | Step::Select { id, .. }
            | Step::Plan { id, .. }
            | Step::MasterPlan { id, .. }
            | Step::Decide { id, .. }
            | Step::Summarize { id, .. }
            | Step::Edit { id, .. }
            | Step::Reply { id, .. }
            | Step::Respond { id, .. }
            | Step::Explore { id, .. }
            | Step::Write { id, .. }
            | Step::Delete { id, .. } => id,
        }
    }

    pub(crate) fn kind(&self) -> &str {
        match self {
            Step::Shell { .. } => "shell",
            Step::Read { .. } => "read",
            Step::Observe { .. } => "observe",
            Step::Search { .. } => "search",
            Step::Select { .. } => "select",
            Step::Plan { .. } => "plan",
            Step::MasterPlan { .. } => "masterplan",
            Step::Decide { .. } => "decide",
            Step::Summarize { .. } => "summarize",
            Step::Edit { .. } => "edit",
            Step::Reply { .. } => "reply",
            Step::Respond { .. } => "respond",
            Step::Explore { .. } => "explore",
            Step::Write { .. } => "write",
            Step::Delete { .. } => "delete",
        }
    }

    pub(crate) fn purpose(&self) -> &str {
        match self {
            Step::Shell { common, .. }
            | Step::Read { common, .. }
            | Step::Observe { common, .. }
            | Step::Search { common, .. }
            | Step::Select { common, .. }
            | Step::Plan { common, .. }
            | Step::MasterPlan { common, .. }
            | Step::Decide { common, .. }
            | Step::Summarize { common, .. }
            | Step::Edit { common, .. }
            | Step::Reply { common, .. }
            | Step::Respond { common, .. }
            | Step::Explore { common, .. }
            | Step::Write { common, .. }
            | Step::Delete { common, .. } => &common.purpose,
        }
    }
}
