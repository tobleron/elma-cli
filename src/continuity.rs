//! @efficiency-role: service-orchestrator
//!
//! Semantic Continuity Tracking — Task 380.
//!
//! Preserves the user's original intent through every pipeline
//! transformation and verifies alignment at key checkpoints:
//! routing → execution → finalization. On drift (< 0.5 alignment),
//! triggers conservative fallback (stricter evidence requirements).

use crate::*;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Whether an intent survived a pipeline transformation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum ContinuityVerdict {
    /// Intent preserved through this stage.
    Aligned,
    /// Intent partially preserved but drifted in specific ways.
    Drifted(String),
    /// Intent was lost or replaced at this stage.
    Mismatch(String),
}

impl ContinuityVerdict {
    pub fn score(&self) -> f64 {
        match self {
            ContinuityVerdict::Aligned => 1.0,
            ContinuityVerdict::Drifted(_) => 0.5,
            ContinuityVerdict::Mismatch(_) => 0.0,
        }
    }
}

/// A single checkpoint in the continuity tracking pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ContinuityCheckpoint {
    pub stage: String,
    pub verdict: ContinuityVerdict,
    pub reason: String,
    pub timestamp_unix: u64,
}

/// Tracks semantic continuity across the entire user-turn pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ContinuityTracker {
    /// Original user intent captured at routing.
    pub original_intent: String,
    pub selected_route: String,
    pub selected_formula: String,
    pub checkpoints: Vec<ContinuityCheckpoint>,
    /// Aggregate alignment score across all checkpoints (0.0–1.0).
    pub alignment_score: f64,
    /// Whether fallback has been triggered.
    pub fallback_triggered: bool,
}

impl ContinuityTracker {
    pub fn new(
        original_intent: String,
        route: &str,
        formula: &str,
    ) -> Self {
        let mut tracker = Self {
            original_intent,
            selected_route: route.to_string(),
            selected_formula: formula.to_string(),
            checkpoints: Vec::new(),
            alignment_score: 1.0,
            fallback_triggered: false,
        };
        tracker.add_checkpoint("initialization", ContinuityVerdict::Aligned, "tracker created");
        tracker
    }

    /// Add a continuity checkpoint and recalculate alignment score.
    pub fn add_checkpoint(
        &mut self,
        stage: &str,
        verdict: ContinuityVerdict,
        reason: &str,
    ) {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.checkpoints.push(ContinuityCheckpoint {
            stage: stage.to_string(),
            verdict,
            reason: reason.to_string(),
            timestamp_unix: ts,
        });
        self.recalculate_score();
    }

    /// Recalculate weighted alignment score across all checkpoints.
    fn recalculate_score(&mut self) {
        if self.checkpoints.is_empty() {
            self.alignment_score = 1.0;
            return;
        }
        let total: f64 = self
            .checkpoints
            .iter()
            .enumerate()
            .map(|(i, cp)| {
                // Later checkpoints are weighted more heavily
                let weight = 1.0 + (i as f64 * 0.5);
                cp.verdict.score() * weight
            })
            .sum();
        let weight_sum: f64 = self
            .checkpoints
            .iter()
            .enumerate()
            .map(|(i, _)| 1.0 + (i as f64 * 0.5))
            .sum();
        self.alignment_score = total / weight_sum;
    }

    /// True if alignment is critically low (requires intervention).
    pub fn needs_fallback(&self) -> bool {
        self.alignment_score < 0.5
    }

    /// True if the most recent checkpoint is fully aligned (not drifted or mismatched).
    pub fn last_checkpoint_is_aligned(&self) -> bool {
        self.checkpoints
            .last()
            .map(|cp| matches!(cp.verdict, ContinuityVerdict::Aligned))
            .unwrap_or(true)
    }

    /// Pre-execution routing check: does the route match a known pattern?
    pub fn check_route_alignment(
        &mut self,
        route_decision: &crate::types_core::RouteDecision,
    ) {
        let route = route_decision.route.to_uppercase();
        let intent_lower = self.original_intent.to_lowercase();

        // Simple factual questions should not use complex routes
        let is_simple_factual = intent_lower.len() < 60
            && !intent_lower.contains("write")
            && !intent_lower.contains("create")
            && !intent_lower.contains("edit")
            && !intent_lower.contains("build");
        let is_complex_route = route == "MASTERPLAN" || route == "PLAN";

        if is_simple_factual && is_complex_route {
            self.add_checkpoint(
                "routing",
                ContinuityVerdict::Drifted(
                    "Simple factual question routed to complex planning route".into(),
                ),
                &format!(
                    "intent='{}' route={} entropy={:.2}",
                    self.original_intent, route, route_decision.entropy
                ),
            );
        } else if route_decision.entropy > 0.8 && route_decision.margin < 0.15 {
            self.add_checkpoint(
                "routing",
                ContinuityVerdict::Drifted(format!(
                    "High entropy ({:.2}) and low margin ({:.2}) suggest uncertain routing",
                    route_decision.entropy, route_decision.margin
                )),
                &format!("entropy={:.2} margin={:.2}", route_decision.entropy, route_decision.margin),
            );
        } else {
            self.add_checkpoint(
                "routing",
                ContinuityVerdict::Aligned,
                &format!("route={} entropy={:.2} margin={:.2}", route, route_decision.entropy, route_decision.margin),
            );
        }
    }

    /// Post-execution check: verify final answer is non-empty and
    /// has a reasonable relationship to the original intent.
    pub fn check_final_answer(
        &mut self,
        final_text: &str,
        has_evidence: bool,
    ) {
        let original_norm: String = self
            .original_intent
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect();
        let answer_norm: String = final_text
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect();

        if final_text.trim().is_empty() {
            self.add_checkpoint(
                "finalization",
                ContinuityVerdict::Mismatch("Final answer is empty".into()),
                "model produced no output",
            );
            return;
        }

        if !has_evidence && original_norm.split_whitespace().count() > 3 {
            self.add_checkpoint(
                "finalization",
                ContinuityVerdict::Drifted(
                    "Final answer has no supporting evidence for non-trivial request".into(),
                ),
                &format!(
                    "original_len={} final_len={}",
                    original_norm.len(),
                    answer_norm.len()
                ),
            );
            return;
        }

        // Check if final answer is too short relative to original (suggests hallucination)
        if !original_norm.is_empty()
            && !answer_norm.is_empty()
            && answer_norm.len() < original_norm.len() / 4
        {
            self.add_checkpoint(
                "finalization",
                ContinuityVerdict::Drifted(format!(
                    "Final answer ({chars} chars) is much shorter than original request ({original} chars). May indicate incomplete response.",
                    chars = answer_norm.len(),
                    original = original_norm.len()
                )),
                &format!(
                    "original_len={} final_len={} ratio={:.2}",
                    original_norm.len(),
                    answer_norm.len(),
                    if original_norm.is_empty() { 1.0 } else { answer_norm.len() as f64 / original_norm.len() as f64 }
                ),
            );
            return;
        }

        self.add_checkpoint(
            "finalization",
            ContinuityVerdict::Aligned,
            &format!(
                "final_len={} has_evidence={}",
                final_text.len(),
                has_evidence
            ),
        );
    }

    /// Mark that a fallback was triggered due to low alignment.
    pub fn trigger_fallback(&mut self) {
        self.fallback_triggered = true;
        self.add_checkpoint(
            "fallback",
            ContinuityVerdict::Drifted("Fallback triggered: low alignment score".into()),
            &format!("alignment_score={:.2}", self.alignment_score),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_continuity_tracker_creation() {
        let ct = ContinuityTracker::new(
            "What time is it?".to_string(),
            "CHAT",
            "direct",
        );
        assert_eq!(ct.original_intent, "What time is it?");
        assert_eq!(ct.selected_route, "CHAT");
        assert_eq!(ct.checkpoints.len(), 1); // initialization checkpoint
        assert!((ct.alignment_score - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_aligned_checkpoint_maintains_score() {
        let mut ct = ContinuityTracker::new("test".into(), "CHAT", "direct");
        ct.add_checkpoint("execution", ContinuityVerdict::Aligned, "OK");
        assert!(ct.alignment_score > 0.9);
    }

    #[test]
    fn test_mismatch_reduces_score() {
        let mut ct = ContinuityTracker::new("test".into(), "CHAT", "direct");
        ct.add_checkpoint("execution", ContinuityVerdict::Mismatch("wrong answer".into()), "bad");
        assert!(ct.alignment_score < 0.5);
    }

    #[test]
    fn test_needs_fallback_on_low_score() {
        let mut ct = ContinuityTracker::new("test".into(), "CHAT", "direct");
        ct.add_checkpoint("execution", ContinuityVerdict::Mismatch("wrong".into()), "fail");
        assert!(ct.needs_fallback());
    }

    #[test]
    fn test_no_fallback_on_high_score() {
        let ct = ContinuityTracker::new("test".into(), "CHAT", "direct");
        assert!(!ct.needs_fallback());
    }

    #[test]
    fn test_verdict_scores() {
        assert_eq!(ContinuityVerdict::Aligned.score(), 1.0);
        assert_eq!(ContinuityVerdict::Drifted("x".into()).score(), 0.5);
        assert_eq!(ContinuityVerdict::Mismatch("x".into()).score(), 0.0);
    }

    #[test]
    fn test_route_alignment_simple_factual() {
        let mut ct = ContinuityTracker::new(
            "What is 2+2?".to_string(),
            "CHAT",
            "direct",
        );
        let route = crate::types_core::RouteDecision {
            route: "CHAT".to_string(),
            source: "test".to_string(),
            distribution: vec![],
            margin: 0.9,
            entropy: 0.1,
            ..Default::default()
        };
        ct.check_route_alignment(&route);
        assert!(ct.last_checkpoint_is_aligned());
    }

    #[test]
    fn test_route_alignment_flags_complex_route() {
        let mut ct = ContinuityTracker::new(
            "What is 2+2?".to_string(),
            "CHAT",
            "direct",
        );
        let route = crate::types_core::RouteDecision {
            route: "MASTERPLAN".to_string(),
            source: "test".to_string(),
            distribution: vec![],
            margin: 0.9,
            entropy: 0.1,
            ..Default::default()
        };
        ct.check_route_alignment(&route);
        assert!(!ct.last_checkpoint_is_aligned());
    }

    #[test]
    fn test_route_alignment_flags_high_entropy() {
        let mut ct = ContinuityTracker::new(
            "Build a web app".to_string(),
            "WORKFLOW",
            "direct",
        );
        let route = crate::types_core::RouteDecision {
            route: "WORKFLOW".to_string(),
            source: "test".to_string(),
            distribution: vec![],
            margin: 0.1,  // low margin = uncertain
            entropy: 0.9, // high entropy
            ..Default::default()
        };
        ct.check_route_alignment(&route);
        assert!(!ct.last_checkpoint_is_aligned());
    }

    #[test]
    fn test_trigger_fallback() {
        let mut ct = ContinuityTracker::new("test".into(), "CHAT", "direct");
        assert!(!ct.fallback_triggered);
        ct.trigger_fallback();
        assert!(ct.fallback_triggered);
        assert_eq!(ct.checkpoints.len(), 2);
    }

    #[test]
    fn test_check_final_answer_empty() {
        let mut ct = ContinuityTracker::new("test".into(), "CHAT", "direct");
        ct.check_final_answer("", true);
        assert!(ct.needs_fallback());
        assert_eq!(ct.checkpoints.last().unwrap().stage, "finalization");
    }

    #[test]
    fn test_check_final_answer_no_evidence() {
        let mut ct = ContinuityTracker::new(
            "Tell me the current time please".into(),
            "CHAT",
            "direct",
        );
        ct.check_final_answer("The time is 5:35 PM.", false);
        assert!(!ct.last_checkpoint_is_aligned());
        assert_eq!(
            ct.checkpoints.last().unwrap().verdict.score(),
            0.5 // Drifted
        );
    }

    #[test]
    fn test_check_final_answer_with_evidence() {
        let mut ct = ContinuityTracker::new(
            "What time is it?".into(),
            "CHAT",
            "direct",
        );
        ct.check_final_answer("It is 5:35 PM.", true);
        assert!(ct.last_checkpoint_is_aligned());
    }

    #[test]
    fn test_check_final_answer_too_short() {
        let mut ct = ContinuityTracker::new(
            "Explain the theory of relativity in detail".into(),
            "CHAT",
            "direct",
        );
        ct.check_final_answer("Yes.", true);
        assert!(!ct.last_checkpoint_is_aligned());
        assert_eq!(
            ct.checkpoints.last().unwrap().verdict.score(),
            0.5 // Drifted (too short)
        );
    }

    #[test]
    fn test_recalculate_score_empty() {
        let mut ct = ContinuityTracker::new("test".into(), "CHAT", "direct");
        ct.checkpoints.clear();
        ct.recalculate_score();
        assert!((ct.alignment_score - 1.0).abs() < 0.01);
    }
}
