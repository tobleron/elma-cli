//! @efficiency-role: service-orchestrator
//!
//! Approach Branch Retry And Prune Engine — Task 390.
//!
//! Wraps pyramid work graph state (Task 389) with approach-aware retry
//! decisions driven by failure classification (Task 379). When an approach
//! fails repeatedly, the engine prunes that branch and generates a new
//! approach toward the same original objective.

use crate::*;
use std::collections::HashMap;

// Re-export work graph types used by this module
pub(crate) use crate::work_graph::{
    ApproachId, ApproachStatus, NodeKind, NodeStatus, WorkGraph, WorkGraphBuilder, WorkNode,
};

/// A single attempt within an approach branch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ApproachAttempt {
    pub approach_id: ApproachId,
    pub attempt_number: u32,
    pub strategy: String,
    pub failure_class: String,
    pub error_summary: String,
    pub timestamp: String,
}

/// Decision returned by the approach engine after recording an attempt.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ApproachDecision {
    /// Continue retrying within the current approach.
    Continue,
    /// Prune the current approach and start a new one.
    PruneAndRetry {
        new_approach_id: ApproachId,
        strategy_hint: String,
        reason: String,
    },
    /// All approaches have been exhausted.
    Exhausted {
        total_approaches: u32,
        reason: String,
    },
}

/// Configuration for the approach engine.
#[derive(Debug, Clone)]
pub(crate) struct ApproachConfig {
    /// Max failures per approach before pruning (default 2).
    pub max_failures_per_approach: u32,
    /// Max total approaches before exhaustion (default 5).
    pub max_total_approaches: u32,
    /// Max total attempts across all approaches (default 10).
    pub max_total_attempts: u32,
}

impl Default for ApproachConfig {
    fn default() -> Self {
        Self {
            max_failures_per_approach: 2,
            max_total_approaches: 5,
            max_total_attempts: 10,
        }
    }
}

/// The approach engine: tracks approach state and makes retry decisions.
pub(crate) struct ApproachEngine {
    graph: WorkGraph,
    config: ApproachConfig,
    attempts: Vec<ApproachAttempt>,
    current_approach_id: ApproachId,
    total_attempts: u32,
}

impl ApproachEngine {
    /// Create a new approach engine for the given user objective.
    pub fn new(objective: String) -> Self {
        let mut builder = WorkGraphBuilder::new(objective);
        let approach_id = builder.approach_id().clone();
        // Add initial root objective node
        builder.add_goal("approach_root", "Primary approach", "Initial approach branch");
        let graph = builder.into_graph();

        Self {
            graph,
            config: ApproachConfig::default(),
            attempts: Vec::new(),
            current_approach_id: approach_id,
            total_attempts: 0,
        }
    }

    /// Create with complexity-gated graph depth.
    /// Uses WorkGraphBuilder::from_complexity to cap pyramid depth.
    pub fn with_complexity(objective: String, complexity: &str) -> Self {
        let mut builder = WorkGraphBuilder::from_complexity(objective, complexity);
        let approach_id = builder.approach_id().clone();

        if !builder.skip_graph() {
            builder.add_goal("approach_root", "Primary approach", "Initial approach branch");
        }

        let graph = builder.into_graph();
        Self {
            graph,
            config: ApproachConfig::default(),
            attempts: Vec::new(),
            current_approach_id: approach_id,
            total_attempts: 0,
        }
    }

    /// Create with custom config.
    pub fn with_config(objective: String, config: ApproachConfig) -> Self {
        let mut engine = Self::new(objective);
        engine.config = config;
        engine
    }

    pub fn graph(&self) -> &WorkGraph {
        &self.graph
    }

    pub fn into_graph(self) -> WorkGraph {
        self.graph
    }

    pub fn attempts(&self) -> &[ApproachAttempt] {
        &self.attempts
    }

    pub fn current_approach_id(&self) -> &ApproachId {
        &self.current_approach_id
    }

    pub fn total_attempts(&self) -> u32 {
        self.total_attempts
    }

    /// Count of currently active (non-failed, non-pruned) approaches.
    pub fn active_approach_count(&self) -> usize {
        self.graph
            .approaches
            .values()
            .filter(|s| **s == ApproachStatus::Active)
            .count()
    }

    /// Total approaches created (including failed and active).
    pub fn total_approach_count(&self) -> usize {
        self.graph.approaches.len()
    }

    /// Record a failed attempt and return a decision for what to do next.
    pub fn record_attempt(
        &mut self,
        strategy: &str,
        failure_class: &str,
        error_summary: &str,
    ) -> ApproachDecision {
        self.total_attempts += 1;

        let attempt = ApproachAttempt {
            approach_id: self.current_approach_id.clone(),
            attempt_number: self.total_attempts,
            strategy: strategy.to_string(),
            failure_class: failure_class.to_string(),
            error_summary: error_summary.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        self.attempts.push(attempt);

        // Check total attempts cap
        if self.total_attempts >= self.config.max_total_attempts {
            self.graph
                .approaches
                .insert(self.current_approach_id.0.clone(), ApproachStatus::Failed);
            return ApproachDecision::Exhausted {
                total_approaches: self.total_approach_count() as u32,
                reason: format!(
                    "Exceeded max total attempts ({})",
                    self.config.max_total_attempts
                ),
            };
        }

        // Count failures in current approach
        let approach_failures = self
            .attempts
            .iter()
            .filter(|a| a.approach_id == self.current_approach_id)
            .count() as u32;

        // Prune if current approach has failed too many times
        if approach_failures >= self.config.max_failures_per_approach {
            // Mark current approach as failed
            self.graph
                .approaches
                .insert(self.current_approach_id.0.clone(), ApproachStatus::Failed);

            // Check total approaches cap (before creating a new one)
            if self.total_approach_count() >= self.config.max_total_approaches as usize {
                return ApproachDecision::Exhausted {
                    total_approaches: self.total_approach_count() as u32,
                    reason: format!(
                        "Exceeded max total approaches ({})",
                        self.config.max_total_approaches
                    ),
                };
            }

            // Generate new approach with different strategy
            let new_id = ApproachId::new();
            let strategy_hint =
                crate::orchestration_retry::strategy_for_failure_by_label(failure_class);
            let reason = format!(
                "Approach failed after {} attempts (failure: {}). Starting new approach.",
                approach_failures, failure_class
            );

            self.graph
                .approaches
                .insert(new_id.0.clone(), ApproachStatus::Active);
            self.current_approach_id = new_id;

            return ApproachDecision::PruneAndRetry {
                new_approach_id: self.current_approach_id.clone(),
                strategy_hint: strategy_hint.to_string(),
                reason,
            };
        }

        ApproachDecision::Continue
    }

    /// Check if there's an active approach.
    pub fn has_active_approach(&self) -> bool {
        self.graph
            .approaches
            .values()
            .any(|s| *s == ApproachStatus::Active)
    }

    /// Fork a new sibling approach branch from the same objective.
    /// Marks the current approach as `Superseded` and creates a new active branch.
    pub fn fork_new_approach(&mut self, reason: &str) -> ApproachId {
        if self.total_approach_count() >= self.config.max_total_approaches as usize {
            return self.current_approach_id.clone();
        }

        self.graph
            .approaches
            .insert(self.current_approach_id.0.clone(), ApproachStatus::Superseded);

        let new_id = ApproachId::new();
        self.graph
            .approaches
            .insert(new_id.0.clone(), ApproachStatus::Active);
        self.current_approach_id = new_id.clone();

        // Add a new root goal for the new approach
        self.graph.add_node(WorkNode {
            id: format!("{}_root", new_id.0),
            kind: NodeKind::Goal,
            label: format!("Forked approach: {}", reason),
            description: reason.to_string(),
            approach_id: new_id.clone(),
            objective: self.graph.root_objective.clone(),
            status: NodeStatus::Pending,
            parent_id: None,
            depth: 0,
        });

        new_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_approach_engine_creation() {
        let engine = ApproachEngine::new("test objective".to_string());
        assert!(engine.has_active_approach());
        assert_eq!(engine.total_attempts(), 0);
        assert_eq!(engine.active_approach_count(), 1);
        assert_eq!(engine.total_approach_count(), 1);
    }

    #[test]
    fn test_record_attempt_continue() {
        let mut engine = ApproachEngine::new("test".to_string());
        let decision = engine.record_attempt("Direct", "json_parse_failure", "parse error");
        assert_eq!(decision, ApproachDecision::Continue);
        assert_eq!(engine.total_attempts(), 1);
        assert_eq!(engine.attempts().len(), 1);
    }

    #[test]
    fn test_approach_prune_after_multiple_failures() {
        let mut engine = ApproachEngine::with_config(
            "test".to_string(),
            ApproachConfig {
                max_failures_per_approach: 2,
                max_total_approaches: 3,
                max_total_attempts: 10,
            },
        );
        let first_id = engine.current_approach_id().clone();

        // First failure: continue
        let d1 = engine.record_attempt("Direct", "parse_error", "fail 1");
        assert_eq!(d1, ApproachDecision::Continue);

        // Second failure: prune
        let d2 = engine.record_attempt("Direct", "parse_error", "fail 2");
        match d2 {
            ApproachDecision::PruneAndRetry { ref reason, .. } => {
                assert!(reason.contains("Approach failed"));
            }
            _ => panic!("Expected PruneAndRetry, got {:?}", d2),
        }

        // Old approach should be Failed
        assert_eq!(
            engine.graph.approaches.get(&first_id.0),
            Some(&ApproachStatus::Failed)
        );

        // New approach should exist and be active
        assert!(engine.has_active_approach());
        assert_ne!(engine.current_approach_id(), &first_id);
    }

    #[test]
    fn test_approach_exhaustion_by_attempts() {
        let mut engine = ApproachEngine::with_config(
            "test".to_string(),
            ApproachConfig {
                max_failures_per_approach: 5,
                max_total_approaches: 5,
                max_total_attempts: 3,
            },
        );

        let d1 = engine.record_attempt("Direct", "timeout", "slow");
        assert_eq!(d1, ApproachDecision::Continue);

        let d2 = engine.record_attempt("Direct", "timeout", "slow");
        assert_eq!(d2, ApproachDecision::Continue);

        let d3 = engine.record_attempt("Direct", "timeout", "slow");
        match d3 {
            ApproachDecision::Exhausted { reason, .. } => {
                assert!(reason.contains("total attempts"));
            }
            _ => panic!("Expected Exhausted, got {:?}", d3),
        }
    }

    #[test]
    fn test_approach_exhaustion_by_approaches() {
        let mut engine = ApproachEngine::with_config(
            "test".to_string(),
            ApproachConfig {
                max_failures_per_approach: 2,
                max_total_approaches: 2,
                max_total_attempts: 10,
            },
        );

        // First approach: 2 failures before pruning
        let d1 = engine.record_attempt("Direct", "timeout", "slow");
        assert_eq!(d1, ApproachDecision::Continue);

        let d2 = engine.record_attempt("Direct", "timeout", "slow");
        assert!(matches!(d2, ApproachDecision::PruneAndRetry { .. }));

        // Second approach: 2 failures -> prune -> exhausted (max_total_approaches=2)
        let d3 = engine.record_attempt("InspectFirst", "timeout", "slow");
        assert_eq!(d3, ApproachDecision::Continue);

        let d4 = engine.record_attempt("InspectFirst", "timeout", "slow");
        match d4 {
            ApproachDecision::Exhausted { reason, .. } => {
                assert!(reason.contains("total approaches"));
            }
            _ => panic!("Expected Exhausted, got {:?}", d4),
        }
    }

    #[test]
    fn test_approach_prune_generates_new_approach() {
        let mut engine = ApproachEngine::with_config(
            "test".to_string(),
            ApproachConfig {
                max_failures_per_approach: 1,
                max_total_approaches: 5,
                max_total_attempts: 10,
            },
        );

        let d1 = engine.record_attempt("Direct", "json_parse_failure", "bad json");
        assert!(matches!(d1, ApproachDecision::PruneAndRetry { .. }));

        if let ApproachDecision::PruneAndRetry {
            ref strategy_hint, ..
        } = d1
        {
            assert!(!strategy_hint.is_empty());
        }

        // Engine still has an active approach (the newly created one)
        assert!(engine.has_active_approach());
        assert_eq!(engine.active_approach_count(), 1);
        assert_eq!(engine.total_approach_count(), 2);
    }

    #[test]
    fn test_approach_engine_custom_config() {
        let config = ApproachConfig {
            max_failures_per_approach: 3,
            max_total_approaches: 10,
            max_total_attempts: 20,
        };
        let engine = ApproachEngine::with_config("test".to_string(), config);
        assert_eq!(engine.config.max_failures_per_approach, 3);
        assert_eq!(engine.config.max_total_approaches, 10);
        assert_eq!(engine.config.max_total_attempts, 20);
    }

    #[test]
    fn test_multiple_approaches_tracked_in_graph() {
        let mut engine = ApproachEngine::with_config(
            "test".to_string(),
            ApproachConfig {
                max_failures_per_approach: 1,
                max_total_approaches: 5,
                max_total_attempts: 10,
            },
        );

        let _ = engine.record_attempt("Direct", "parse_error", "fail 1");
        let _ = engine.record_attempt("InspectFirst", "parse_error", "fail 2");
        let _ = engine.record_attempt("PlanThenExecute", "parse_error", "fail 3");

        // 3 attempts = 3 failed + 1 active (the current one)
        let failed_count = engine
            .graph
            .approaches
            .values()
            .filter(|s| **s == ApproachStatus::Failed)
            .count();
        let active_count = engine
            .graph
            .approaches
            .values()
            .filter(|s| **s == ApproachStatus::Active)
            .count();
        assert_eq!(failed_count, 3);
        assert_eq!(active_count, 1);
        assert_eq!(engine.total_approach_count(), 4);
    }

    #[test]
    fn test_record_attempt_stores_correct_data() {
        let mut engine = ApproachEngine::new("test".to_string());
        let _ = engine.record_attempt("SafeMode", "empty_output", "no output produced");
        let attempt = &engine.attempts()[0];
        assert_eq!(attempt.strategy, "SafeMode");
        assert_eq!(attempt.failure_class, "empty_output");
        assert_eq!(attempt.error_summary, "no output produced");
        assert_eq!(attempt.attempt_number, 1);
    }
}
