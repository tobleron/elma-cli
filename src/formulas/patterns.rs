//! @efficiency-role: data-model
//! Formula Patterns - Abstract intent patterns (NO hardcoded commands)

use serde::{Deserialize, Serialize};

/// Formula Pattern - Abstract intent, not commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormulaPattern {
    /// Formula name (e.g., "reply_only", "inspect_reply")
    pub name: &'static str,

    /// Human-readable description of intent
    pub description: &'static str,

    /// What this formula means (abstract)
    pub intent: &'static str,

    /// Expected step types (abstract, not specific commands)
    pub expected_step_types: Vec<&'static str>,

    /// Expected number of steps
    pub expected_steps: usize,

    /// When to use this formula
    pub use_cases: Vec<&'static str>,

    /// When NOT to use this formula
    pub anti_patterns: Vec<&'static str>,
}

impl FormulaPattern {
    /// Get all available formula patterns
    pub fn all() -> Vec<FormulaPattern> {
        vec![
            FormulaPattern::reply_only(),
            FormulaPattern::inspect_reply(),
            FormulaPattern::inspect_summarize_reply(),
            FormulaPattern::inspect_decide_reply(),
            FormulaPattern::inspect_edit_verify_reply(),
            FormulaPattern::plan_reply(),
            FormulaPattern::masterplan_reply(),
        ]
    }

    /// Find formula by name
    pub fn by_name(name: &str) -> Option<FormulaPattern> {
        Self::all().into_iter().find(|f| f.name == name)
    }

    // ========================================================================
    // Formula Definitions (Abstract Patterns - NO Commands)
    // ========================================================================

    /// Reply Only - Answer directly without inspection
    ///
    /// **Intent:** Provide immediate answer from knowledge
    /// **Steps:** 1 (reply)
    /// **Best for:** Greetings, simple Q&A, capability questions
    fn reply_only() -> Self {
        FormulaPattern {
            name: "reply_only",
            description: "Answer directly without workspace inspection",
            intent: "Provide immediate answer from knowledge or conversation context",
            expected_step_types: vec!["reply"],
            expected_steps: 1,
            use_cases: vec![
                "Greetings (hello, hi)",
                "Simple Q&A (what is X)",
                "Capability questions (can you do X)",
                "Conversational turns",
            ],
            anti_patterns: vec![
                "Questions about workspace files",
                "Requests requiring evidence",
                "Complex multi-part questions",
            ],
        }
    }

    /// Inspect Reply - Inspect workspace then answer
    ///
    /// **Intent:** Gather evidence, then provide answer
    /// **Steps:** 2 (inspect + reply)
    /// **Best for:** File lookup, quick facts, simple searches
    fn inspect_reply() -> Self {
        FormulaPattern {
            name: "inspect_reply",
            description: "Inspect workspace evidence, then reply with findings",
            intent: "Gather workspace evidence through inspection, then answer based on findings",
            expected_step_types: vec!["inspect", "reply"],
            expected_steps: 2,
            use_cases: vec![
                "File content questions (what does X contain)",
                "Simple searches (where is X defined)",
                "Quick facts about project",
                "List/show/print requests",
            ],
            anti_patterns: vec![
                "Complex multi-file analysis",
                "Requests requiring decisions",
                "Edit/modify requests",
            ],
        }
    }

    /// Inspect Summarize Reply - Inspect, summarize, then answer
    ///
    /// **Intent:** Gather evidence, organize it, then present summary
    /// **Steps:** 3 (inspect + summarize + reply)
    /// **Best for:** Project overviews, summaries, structured information
    fn inspect_summarize_reply() -> Self {
        FormulaPattern {
            name: "inspect_summarize_reply",
            description: "Inspect workspace, summarize findings, then reply",
            intent: "Gather evidence, organize/summarize it meaningfully, then present structured answer",
            expected_step_types: vec!["inspect", "summarize", "reply"],
            expected_steps: 3,
            use_cases: vec![
                "Project structure questions",
                "Summaries (summarize X)",
                "Overviews (what's in this project)",
                "Organized information requests",
            ],
            anti_patterns: vec![
                "Simple yes/no questions",
                "Single file lookups",
                "Requests requiring decisions",
            ],
        }
    }

    /// Inspect Decide Reply - Inspect, make decision, then answer
    ///
    /// **Intent:** Gather evidence, evaluate options, recommend course of action
    /// **Steps:** 3 (inspect + decide + reply)
    /// **Best for:** Recommendations, choices, evaluations
    fn inspect_decide_reply() -> Self {
        FormulaPattern {
            name: "inspect_decide_reply",
            description:
                "Inspect workspace, make decision/evaluation, then reply with recommendation",
            intent:
                "Gather evidence, evaluate options or make judgment, then provide recommendation",
            expected_step_types: vec!["inspect", "decide", "reply"],
            expected_steps: 3,
            use_cases: vec![
                "Recommendations (what should I use for X)",
                "Evaluations (is X good)",
                "Choices (which approach is better)",
                "Best practice questions",
            ],
            anti_patterns: vec![
                "Simple factual questions",
                "Edit/modify requests",
                "Requests requiring implementation",
            ],
        }
    }

    /// Inspect Edit Verify Reply - Read, modify, verify, then answer
    ///
    /// **Intent:** Read current state, make changes, verify correctness, report results
    /// **Steps:** 4 (read + edit + verify + reply)
    /// **Best for:** Code changes, fixes, modifications
    fn inspect_edit_verify_reply() -> Self {
        FormulaPattern {
            name: "inspect_edit_verify_reply",
            description: "Read file, make edits, verify changes, then reply with results",
            intent: "Read current content, apply modifications, verify correctness, report outcome",
            expected_step_types: vec!["read", "edit", "verify", "reply"],
            expected_steps: 4,
            use_cases: vec![
                "Code fixes (fix bug in X)",
                "Modifications (change X to Y)",
                "Additions (add feature X)",
                "Refactoring (improve X)",
            ],
            anti_patterns: vec![
                "Read-only questions",
                "Simple lookups",
                "Questions without modification needs",
            ],
        }
    }

    /// Plan Reply - Create implementation plan then answer
    ///
    /// **Intent:** Analyze requirements, create step-by-step plan, present it
    /// **Steps:** 2 (plan + reply)
    /// **Best for:** Implementation plans, how-to guides
    fn plan_reply() -> Self {
        FormulaPattern {
            name: "plan_reply",
            description: "Create step-by-step implementation plan, then reply with plan",
            intent: "Analyze objective, break into actionable steps, present implementation plan",
            expected_step_types: vec!["plan", "reply"],
            expected_steps: 2,
            use_cases: vec![
                "Implementation requests (how do I build X)",
                "Step-by-step guides",
                "Process questions (what's the process for X)",
                "Task planning",
            ],
            anti_patterns: vec![
                "Simple factual questions",
                "Requests for immediate action",
                "Strategic/long-term planning",
            ],
        }
    }

    /// Masterplan Reply - Create strategic plan then answer
    ///
    /// **Intent:** Analyze complex objective, create multi-phase strategic plan, present it
    /// **Steps:** 2 (masterplan + reply)
    /// **Best for:** Complex multi-phase work, strategic planning
    fn masterplan_reply() -> Self {
        FormulaPattern {
            name: "masterplan_reply",
            description: "Create high-level strategic plan with phases/milestones, then reply",
            intent: "Analyze complex objective, identify phases and milestones, create strategic roadmap",
            expected_step_types: vec!["masterplan", "reply"],
            expected_steps: 2,
            use_cases: vec![
                "Complex multi-phase projects",
                "Strategic planning (how to migrate X to Y)",
                "Long-term roadmaps",
                "Architecture decisions",
            ],
            anti_patterns: vec![
                "Simple tasks",
                "Single-phase work",
                "Immediate action requests",
            ],
        }
    }
}

/// Formula Selection - Result of formula matching
#[derive(Debug, Clone)]
pub struct FormulaSelection {
    /// Selected formula
    pub formula: FormulaPattern,

    /// Confidence in selection (0.0 - 1.0)
    pub confidence: f32,

    /// Why this formula was selected
    pub reason: String,

    /// Alternative formulas considered
    pub alternatives: Vec<String>,
}

/// Match formula to user request based on complexity and intent
pub fn match_formula_to_request(
    user_intent: &str,
    complexity: &str,
    risk: &str,
) -> FormulaSelection {
    // Simple matching logic (can be enhanced with ML later)
    let (formula, confidence, reason) = match (complexity, risk) {
        ("DIRECT", "LOW") => {
            if user_intent.contains("greet") || user_intent.contains("who are you") {
                (
                    FormulaPattern::reply_only(),
                    0.95,
                    "Simple conversational turn",
                )
            } else {
                (
                    FormulaPattern::inspect_reply(),
                    0.85,
                    "Direct request needing evidence",
                )
            }
        }
        ("INVESTIGATE", _) => {
            if user_intent.contains("summarize") || user_intent.contains("overview") {
                (
                    FormulaPattern::inspect_summarize_reply(),
                    0.90,
                    "Summary/overview request",
                )
            } else {
                (
                    FormulaPattern::inspect_reply(),
                    0.80,
                    "Investigation request",
                )
            }
        }
        ("MULTISTEP", "HIGH") => (
            FormulaPattern::inspect_edit_verify_reply(),
            0.90,
            "Complex task with high risk",
        ),
        ("MULTISTEP", _) => {
            if user_intent.contains("plan") || user_intent.contains("how to") {
                (FormulaPattern::plan_reply(), 0.85, "Planning request")
            } else {
                (
                    FormulaPattern::inspect_edit_verify_reply(),
                    0.80,
                    "Multi-step task",
                )
            }
        }
        ("OPEN_ENDED", _) => (
            FormulaPattern::masterplan_reply(),
            0.90,
            "Complex strategic request",
        ),
        _ => (FormulaPattern::inspect_reply(), 0.70, "Default formula"),
    };

    FormulaSelection {
        formula,
        confidence,
        reason: reason.to_string(),
        alternatives: vec![
            "inspect_reply".to_string(),
            "inspect_summarize_reply".to_string(),
        ],
    }
}
