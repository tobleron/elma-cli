//! @efficiency-role: domain-logic
//! Model-signal complexity gate for maximum work graph depth.
//!
//! Semantic classification belongs to the model/intel unit. This module only
//! normalizes that signal, assigns the depth ceiling, and provides a conservative
//! shape-based fallback when no model signal is available.
//!
//! Scope-based reassessment upgrades complexity when workspace discovery reveals
//! a large file set that the input shape alone could not predict.

use std::collections::HashMap;

/// Complexity level for a user request, used to gate work graph depth.
/// Two gates: Direct (no graph) or Multistep (Objective → SubGoal → Instruction).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ComplexityLevel {
    Direct,
    Multistep,
}

/// Assessment result with confidence score, reasoning, and derived max depth.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ComplexityAssessment {
    pub(crate) level: ComplexityLevel,
    pub(crate) confidence: f32,
    pub(crate) reasoning: String,
    pub(crate) max_graph_depth: usize,
}

impl ComplexityAssessment {
    pub(crate) fn to_types_api(&self) -> crate::types_api::ComplexityAssessment {
        crate::types_api::ComplexityAssessment {
            complexity: match self.level {
                ComplexityLevel::Direct => "DIRECT".to_string(),
                ComplexityLevel::Multistep => "MULTISTEP".to_string(),
            },
            needs_evidence: true,
            needs_tools: true,
            needs_decision: true,
            needs_plan: self.level == ComplexityLevel::Multistep,
            risk: "LOW".to_string(),
            suggested_pattern: "AUTO".to_string(),
        }
    }
}

/// Model-provided complexity signal after strict JSON/intel-unit parsing.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ModelComplexitySignal {
    pub(crate) level: ComplexityLevel,
    pub(crate) confidence: f32,
    pub(crate) reasoning: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InputShape {
    words: usize,
    lines: usize,
    structural_units: usize,
}

/// Gate that converts model signals into hard graph-depth ceilings.
#[derive(Debug, Clone)]
pub(crate) struct ComplexityGate;

impl ComplexityGate {
    /// Conservative fallback for contexts where the model signal is unavailable.
    /// This intentionally uses only request shape, not hardcoded intent words.
    pub(crate) fn assess(input: &str, context_hint: Option<&str>) -> ComplexityAssessment {
        let shape = InputShape::from(input);
        let level = fallback_level(shape);
        ComplexityAssessment {
            level,
            confidence: fallback_confidence(shape, level),
            reasoning: fallback_reasoning(shape, context_hint),
            max_graph_depth: Self::max_depth_for_level(&level),
        }
    }

    /// Normalize a model-provided semantic assessment into the runtime depth
    /// contract. If the signal is contradictory to request shape, keep the
    /// model's level but lower confidence so callers can re-assess.
    pub(crate) fn assess_model_signal(
        input: &str,
        signal: ModelComplexitySignal,
        context_hint: Option<&str>,
    ) -> ComplexityAssessment {
        let shape = InputShape::from(input);
        let level = signal.level;
        let confidence = signal.confidence.clamp(0.0, 1.0);
        let confidence = if shape_supports_level(shape, level) {
            confidence
        } else {
            confidence.min(0.45)
        };
        let context = context_hint
            .filter(|hint| !hint.trim().is_empty())
            .map(|hint| format!(" context={}", hint.trim()))
            .unwrap_or_default();
        let reasoning = if signal.reasoning.trim().is_empty() {
            format!(
                "model_signal normalized with words={} lines={} units={}{}",
                shape.words, shape.lines, shape.structural_units, context
            )
        } else {
            format!(
                "model_signal: {}; words={} lines={} units={}{}",
                signal.reasoning.trim(),
                shape.words,
                shape.lines,
                shape.structural_units,
                context
            )
        };

        ComplexityAssessment {
            level,
            confidence,
            reasoning,
            max_graph_depth: Self::max_depth_for_level(&level),
        }
    }

    /// Return the maximum work graph depth for a given complexity level.
    pub(crate) fn max_depth(&self, level: &ComplexityLevel) -> usize {
        Self::max_depth_for_level(level)
    }

    fn max_depth_for_level(level: &ComplexityLevel) -> usize {
        match level {
            ComplexityLevel::Direct => 0,
            ComplexityLevel::Multistep => 2, // Objective(0) → SubGoal(1) → Instruction(2)
        }
    }

    /// Scope-based reassessment: upgrade complexity when workspace discovery
    /// reveals a file set larger than the input shape suggested.
    ///
    /// Scope signals are grounded counts (files, file-type variety, estimated
    /// work items). No keyword matching. The upgrade ladder is conservative:
    /// each call upgrades at most one level. Multistep is the ceiling.
    pub(crate) fn reassess_with_scope(
        previous_level: ComplexityLevel,
        discovered_files: usize,
        file_type_mix: &HashMap<String, usize>,
        estimated_work_items: usize,
    ) -> ComplexityAssessment {
        let upgrade = discovered_files > 10 || estimated_work_items > 20;
        if !upgrade {
            return ComplexityAssessment {
                level: previous_level,
                confidence: 0.70,
                reasoning: format!(
                    "scope_reassessment no_upgrade files={} work_items={}",
                    discovered_files, estimated_work_items
                ),
                max_graph_depth: Self::max_depth_for_level(&previous_level),
            };
        }

        let new_level = match previous_level {
            ComplexityLevel::Direct => ComplexityLevel::Multistep,
            ComplexityLevel::Multistep => ComplexityLevel::Multistep, // already at ceiling
        };

        let confidence = if discovered_files > 50 || estimated_work_items > 100 {
            0.80
        } else {
            0.62
        };

        ComplexityAssessment {
            level: new_level,
            confidence,
            reasoning: format!(
                "scope_reassessment upgrade {:?}->{:?} files={} file_types={} work_items={}",
                previous_level, new_level, discovered_files, file_type_mix.len(), estimated_work_items
            ),
            max_graph_depth: Self::max_depth_for_level(&new_level),
        }
    }

    /// Validate only structural plausibility. Semantic continuity must be
    /// checked against the model/intel-unit chain, not duplicated here.
    pub(crate) fn validate_continuity(input: &str, assessment: &ComplexityAssessment) -> bool {
        shape_supports_level(InputShape::from(input), assessment.level)
    }
}

impl InputShape {
    fn from(input: &str) -> Self {
        let words = input.split_whitespace().count();
        let lines = input.lines().filter(|line| !line.trim().is_empty()).count();
        let sentence_breaks = input
            .chars()
            .filter(|ch| matches!(ch, '.' | '?' | '!' | ';' | ':'))
            .count();
        let list_markers = input
            .lines()
            .filter(|line| {
                let trimmed = line.trim_start();
                trimmed.starts_with("- ")
                    || trimmed.starts_with("* ")
                    || trimmed.chars().next().is_some_and(|ch| ch.is_ascii_digit())
                        && trimmed.get(1..3).is_some_and(|tail| tail == ". ")
            })
            .count();
        let structural_units = lines.max(sentence_breaks + list_markers).max(1);
        Self {
            words,
            lines,
            structural_units,
        }
    }
}

fn fallback_level(shape: InputShape) -> ComplexityLevel {
    if shape.words == 0 || (shape.words <= 6 && shape.structural_units <= 1) {
        ComplexityLevel::Direct
    } else {
        ComplexityLevel::Multistep
    }
}

fn fallback_confidence(shape: InputShape, level: ComplexityLevel) -> f32 {
    let base = match level {
        ComplexityLevel::Direct => 0.70,
        ComplexityLevel::Multistep => 0.58,
    };
    if shape.words == 0 {
        0.40
    } else {
        base
    }
}

fn fallback_reasoning(shape: InputShape, context_hint: Option<&str>) -> String {
    let context = context_hint
        .filter(|hint| !hint.trim().is_empty())
        .map(|hint| format!(" context={}", hint.trim()))
        .unwrap_or_default();
    format!(
        "shape_fallback words={} lines={} units={}{}",
        shape.words, shape.lines, shape.structural_units, context
    )
}

fn shape_supports_level(shape: InputShape, level: ComplexityLevel) -> bool {
    match level {
        ComplexityLevel::Direct => shape.words <= 6 && shape.lines <= 1,
        ComplexityLevel::Multistep => shape.words >= 6 || shape.structural_units >= 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_keeps_short_input_direct() {
        let result = ComplexityGate::assess("hello", None);
        assert_eq!(result.level, ComplexityLevel::Direct);
        assert_eq!(result.max_graph_depth, 0);
        assert!(result.confidence <= 0.70);
    }

    #[test]
    fn fallback_uses_request_shape_without_keyword_triggers() {
        let result = ComplexityGate::assess(
            "Review the parser changes. Check session persistence. Confirm tests.",
            None,
        );
        assert_eq!(result.level, ComplexityLevel::Multistep);
        assert_eq!(result.max_graph_depth, 2);
    }

    #[test]
    fn fallback_caps_long_input_as_multistep() {
        let input = (0..90).map(|_| "word").collect::<Vec<_>>().join(" ");
        let result = ComplexityGate::assess(&input, None);
        assert_eq!(result.level, ComplexityLevel::Multistep);
        assert_eq!(result.max_graph_depth, 2);
    }

    #[test]
    fn model_signal_owns_semantic_level() {
        let signal = ModelComplexitySignal {
            level: ComplexityLevel::Multistep,
            confidence: 0.91,
            reasoning: "multi-step task".into(),
        };
        let result = ComplexityGate::assess_model_signal(
            "inspect this entire module thoroughly for all issues",
            signal,
            None,
        );
        assert_eq!(result.level, ComplexityLevel::Multistep);
        assert_eq!(result.max_graph_depth, 2);
        assert_eq!(result.confidence, 0.91);
    }

    #[test]
    fn contradictory_model_signal_is_downweighted_not_overwritten() {
        let signal = ModelComplexitySignal {
            level: ComplexityLevel::Multistep,
            confidence: 0.95,
            reasoning: "broad objective".into(),
        };
        let result = ComplexityGate::assess_model_signal("hi", signal, None);
        assert_eq!(result.level, ComplexityLevel::Multistep);
        assert_eq!(result.confidence, 0.45);
    }

    #[test]
    fn max_depth_method() {
        let gate = ComplexityGate;
        assert_eq!(gate.max_depth(&ComplexityLevel::Direct), 0);
        assert_eq!(gate.max_depth(&ComplexityLevel::Multistep), 2);
    }

    #[test]
    fn structural_continuity_rejects_direct_for_large_request() {
        let assessment = ComplexityAssessment {
            level: ComplexityLevel::Direct,
            confidence: 0.95,
            reasoning: "short input".into(),
            max_graph_depth: 0,
        };
        assert!(!ComplexityGate::validate_continuity(
            "please refactor the entire authentication system across all files",
            &assessment,
        ));
    }

    // ── Scope-based reassessment tests (Task 760) ───────────────────────────

    #[test]
    fn scope_reassessment_small_scope_stays() {
        let mix = std::collections::HashMap::new();
        let result = ComplexityGate::reassess_with_scope(
            ComplexityLevel::Multistep,
            3,
            &mix,
            6,
        );
        assert_eq!(result.level, ComplexityLevel::Multistep);
        assert_eq!(result.max_graph_depth, 2);
    }

    #[test]
    fn scope_reassessment_direct_small_scope_stays_direct() {
        let mix = HashMap::new();
        let result = ComplexityGate::reassess_with_scope(
            ComplexityLevel::Direct,
            5,
            &mix,
            10,
        );
        assert_eq!(result.level, ComplexityLevel::Direct);
        assert_eq!(result.max_graph_depth, 0);
    }

    #[test]
    fn scope_reassessment_large_scope_upgrades_direct_to_multistep() {
        let mut mix = HashMap::new();
        mix.insert("rs".to_string(), 8);
        mix.insert("toml".to_string(), 2);
        let result = ComplexityGate::reassess_with_scope(
            ComplexityLevel::Direct,
            15,
            &mix,
            30,
        );
        assert_eq!(result.level, ComplexityLevel::Multistep);
        assert_eq!(result.max_graph_depth, 2);
        assert!(result.reasoning.contains("upgrade"));
    }

    #[test]
    fn scope_reassessment_large_scope_stays_multistep() {
        let mut mix = HashMap::new();
        mix.insert("md".to_string(), 30);
        let result = ComplexityGate::reassess_with_scope(
            ComplexityLevel::Multistep,
            44,
            &mix,
            88,
        );
        assert_eq!(result.level, ComplexityLevel::Multistep);
        assert_eq!(result.max_graph_depth, 2);
    }

    #[test]
    fn scope_reassessment_small_scope_stays_multistep() {
        let mix = HashMap::new();
        let result = ComplexityGate::reassess_with_scope(
            ComplexityLevel::Multistep,
            2,
            &mix,
            4,
        );
        assert_eq!(result.level, ComplexityLevel::Multistep);
    }
}

pub(crate) fn complexity_level_label(level: ComplexityLevel) -> &'static str {
    match level {
        ComplexityLevel::Direct => "DIRECT",
        ComplexityLevel::Multistep => "MULTISTEP",
    }
}

pub(crate) fn max_iter_for_level(level: ComplexityLevel) -> usize {
    match level {
        ComplexityLevel::Direct => 3,
        ComplexityLevel::Multistep => 10,
    }
}
