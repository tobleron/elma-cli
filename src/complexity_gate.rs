//! @efficiency-role: domain-logic
//! Model-signal complexity gate for maximum work graph depth.
//!
//! Semantic classification belongs to the model/intel unit. This module only
//! normalizes that signal, assigns the depth ceiling, and provides a conservative
//! shape-based fallback when no model signal is available.

/// Complexity level for a user request, used to gate work graph depth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ComplexityLevel {
    Direct,
    Investigate,
    Multistep,
    OpenEnded,
}

/// Assessment result with confidence score, reasoning, and derived max depth.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ComplexityAssessment {
    pub(crate) level: ComplexityLevel,
    pub(crate) confidence: f32,
    pub(crate) reasoning: String,
    pub(crate) max_graph_depth: usize,
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
            ComplexityLevel::Investigate => 2,
            ComplexityLevel::Multistep => 3,
            ComplexityLevel::OpenEnded => 4,
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
    } else if shape.lines > 4 || shape.words > 80 {
        ComplexityLevel::OpenEnded
    } else if shape.structural_units >= 3 || shape.words > 24 {
        ComplexityLevel::Multistep
    } else {
        ComplexityLevel::Investigate
    }
}

fn fallback_confidence(shape: InputShape, level: ComplexityLevel) -> f32 {
    let base = match level {
        ComplexityLevel::Direct => 0.70,
        ComplexityLevel::Investigate => 0.62,
        ComplexityLevel::Multistep => 0.58,
        ComplexityLevel::OpenEnded => 0.52,
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
        ComplexityLevel::Investigate => shape.words > 0 && shape.words <= 80 && shape.lines <= 4,
        ComplexityLevel::Multistep => shape.words >= 6 || shape.structural_units >= 2,
        ComplexityLevel::OpenEnded => {
            shape.words >= 12 || shape.lines >= 3 || shape.structural_units >= 3
        }
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
        assert_eq!(result.max_graph_depth, 3);
    }

    #[test]
    fn fallback_caps_long_open_shape() {
        let input = (0..90).map(|_| "word").collect::<Vec<_>>().join(" ");
        let result = ComplexityGate::assess(&input, None);
        assert_eq!(result.level, ComplexityLevel::OpenEnded);
        assert_eq!(result.max_graph_depth, 4);
    }

    #[test]
    fn model_signal_owns_semantic_level() {
        let signal = ModelComplexitySignal {
            level: ComplexityLevel::Investigate,
            confidence: 0.91,
            reasoning: "bounded inspection".into(),
        };
        let result = ComplexityGate::assess_model_signal("inspect this module", signal, None);
        assert_eq!(result.level, ComplexityLevel::Investigate);
        assert_eq!(result.max_graph_depth, 2);
        assert_eq!(result.confidence, 0.91);
    }

    #[test]
    fn contradictory_model_signal_is_downweighted_not_overwritten() {
        let signal = ModelComplexitySignal {
            level: ComplexityLevel::OpenEnded,
            confidence: 0.95,
            reasoning: "broad objective".into(),
        };
        let result = ComplexityGate::assess_model_signal("hi", signal, None);
        assert_eq!(result.level, ComplexityLevel::OpenEnded);
        assert_eq!(result.confidence, 0.45);
    }

    #[test]
    fn max_depth_method() {
        let gate = ComplexityGate;
        assert_eq!(gate.max_depth(&ComplexityLevel::Direct), 0);
        assert_eq!(gate.max_depth(&ComplexityLevel::Investigate), 2);
        assert_eq!(gate.max_depth(&ComplexityLevel::Multistep), 3);
        assert_eq!(gate.max_depth(&ComplexityLevel::OpenEnded), 4);
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
}
