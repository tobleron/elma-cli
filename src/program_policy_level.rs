//! @efficiency-role: domain-logic
//!
//! Program Policy Level Validation
//!
//! Task 044: Added execution level validation.

use crate::execution_ladder::ExecutionLevel;
use crate::*;

/// Detect duplicate step ratio in a program
///
/// Task 014: Returns the ratio of duplicate steps (0.0 to 1.0).
/// High ratio indicates a planning loop.
pub(crate) fn detect_duplicate_step_ratio(program: &Program) -> f64 {
    if program.steps.len() < 2 {
        return 0.0;
    }

    // Group steps by (type, cmd/content, purpose) signature
    let mut step_signatures: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    for step in &program.steps {
        let signature = match step {
            Step::Shell { cmd, common, .. } => {
                format!("shell:{}:{}", cmd, common.purpose)
            }
            Step::Read { path, common, .. } => {
                format!("read:{}:{}", path, common.purpose)
            }
            Step::Search {
                query,
                paths,
                common,
                ..
            } => {
                format!("search:{}:{:?}", query, common.purpose)
            }
            Step::Select {
                instructions,
                common,
                ..
            } => {
                format!("select:{}:{}", instructions, common.purpose)
            }
            Step::Plan { goal, common, .. } => {
                format!("plan:{}:{}", goal, common.purpose)
            }
            Step::MasterPlan { goal, common, .. } => {
                format!("masterplan:{}:{}", goal, common.purpose)
            }
            Step::Decide { prompt, common, .. } => {
                format!("decide:{}:{}", prompt, common.purpose)
            }
            Step::Summarize {
                instructions,
                common,
                ..
            } => {
                format!("summarize:{}:{}", instructions, common.purpose)
            }
            Step::Edit { spec, common, .. } => {
                format!("edit:{}:{}:{}", spec.operation, spec.path, common.purpose)
            }
            Step::Reply {
                instructions,
                common,
                ..
            } => {
                format!("reply:{}:{}", instructions, common.purpose)
            }
            Step::Respond {
                instructions,
                common,
                ..
            } => format!("respond:{}:{}", instructions, common.purpose),
            Step::Explore {
                objective, common, ..
            } => format!("explore:{}:{}", objective, common.purpose),
            Step::Write {
                path,
                content,
                common,
                ..
            } => format!(
                "write:{}:{}:{}",
                path,
                content.chars().take(50).collect::<String>(),
                common.purpose
            ),
            Step::Delete { path, common, .. } => format!("delete:{}:{}", path, common.purpose),
        };

        *step_signatures.entry(signature).or_insert(0) += 1;
    }

    // Count duplicates (steps that appear more than once)
    let duplicate_count: usize = step_signatures.values().filter(|&&count| count > 1).sum();

    duplicate_count as f64 / program.steps.len() as f64
}

/// Check if a program matches the required execution level
///
/// Validates that the program is neither overbuilt nor underbuilt for the level.
/// Also enforces hard step limits to prevent plan collapse (40+ identical steps).
pub fn program_matches_level(
    program: &Program,
    required_level: ExecutionLevel,
) -> Result<(), PolicyError> {
    let has_plan = program.steps.iter().any(|s| matches!(s, Step::Plan { .. }));
    let has_masterplan = program
        .steps
        .iter()
        .any(|s| matches!(s, Step::MasterPlan { .. }));
    let step_count = program.steps.len();
    let has_reply = program
        .steps
        .iter()
        .any(|s| matches!(s, Step::Reply { .. }));

    // Task 014: Hard maximum step limit to prevent plan collapse
    // No program should exceed 12 steps - if it does, it's likely a loop
    const MAX_STEPS_ABSOLUTE: usize = 12;
    if step_count > MAX_STEPS_ABSOLUTE {
        return Err(PolicyError::MaxStepsExceeded {
            count: step_count,
            max: MAX_STEPS_ABSOLUTE,
        });
    }

    // Task 014: Detect duplicate step loops
    // If >50% of steps are duplicates, reject as a loop
    if step_count >= 4 {
        let duplicate_ratio = detect_duplicate_step_ratio(program);
        if duplicate_ratio > 0.5 {
            return Err(PolicyError::DuplicateStepRatio {
                ratio: (duplicate_ratio * 100.0) as usize,
            });
        }
    }

    match required_level {
        ExecutionLevel::Action => {
            // Action level: should be 1-2 steps (primary action + reply)
            // Reject if has Plan/MasterPlan structure
            if has_plan {
                return Err(PolicyError::InvalidLevelStructure {
                    level: ExecutionLevel::Action,
                    structure: "Plan".to_string(),
                });
            }
            if has_masterplan {
                return Err(PolicyError::InvalidLevelStructure {
                    level: ExecutionLevel::Action,
                    structure: "MasterPlan".to_string(),
                });
            }
            // Allow 1-3 steps (action + optional evidence + reply)
            if step_count > 3 {
                return Err(PolicyError::StepCountMismatch {
                    level: ExecutionLevel::Action,
                    bound: "many".to_string(),
                    count: step_count,
                    expected: "1-3".to_string(),
                });
            }
        }

        ExecutionLevel::Task => {
            // Task level: bounded outcome, 2-6 steps typical
            // Reject if has Plan/MasterPlan structure
            if has_plan {
                return Err(PolicyError::InvalidLevelStructure {
                    level: ExecutionLevel::Task,
                    structure: "Plan".to_string(),
                });
            }
            if has_masterplan {
                return Err(PolicyError::InvalidLevelStructure {
                    level: ExecutionLevel::Task,
                    structure: "MasterPlan".to_string(),
                });
            }
            // Allow 2-8 steps (evidence chain + transformation + reply)
            if step_count < 2 {
                return Err(PolicyError::StepCountMismatch {
                    level: ExecutionLevel::Task,
                    bound: "few".to_string(),
                    count: step_count,
                    expected: "2-8".to_string(),
                });
            }
            if step_count > 8 {
                return Err(PolicyError::StepCountMismatch {
                    level: ExecutionLevel::Task,
                    bound: "many".to_string(),
                    count: step_count,
                    expected: "2-8".to_string(),
                });
            }
        }

        ExecutionLevel::Plan => {
            // Plan level: must have explicit Plan step
            if !has_plan {
                return Err(PolicyError::MissingRequiredStep {
                    level: ExecutionLevel::Plan,
                    step_type: "Plan".to_string(),
                });
            }
            // Should have reasonable structure (Plan + supporting steps + reply)
            // Task 014: Add upper bound to prevent explosion
            if step_count < 2 {
                return Err(PolicyError::StepCountMismatch {
                    level: ExecutionLevel::Plan,
                    bound: "few".to_string(),
                    count: step_count,
                    expected: "2-10".to_string(),
                });
            }
            if step_count > 10 {
                return Err(PolicyError::StepCountMismatch {
                    level: ExecutionLevel::Plan,
                    bound: "many".to_string(),
                    count: step_count,
                    expected: "2-10".to_string(),
                });
            }
        }

        ExecutionLevel::MasterPlan => {
            // MasterPlan level: must have explicit MasterPlan step
            if !has_masterplan {
                return Err(PolicyError::MissingRequiredStep {
                    level: ExecutionLevel::MasterPlan,
                    step_type: "MasterPlan".to_string(),
                });
            }
            // Should have strategic structure (MasterPlan + phases + reply)
            // Task 014: Add upper bound to prevent explosion
            if step_count < 2 {
                return Err(PolicyError::StepCountMismatch {
                    level: ExecutionLevel::MasterPlan,
                    bound: "few".to_string(),
                    count: step_count,
                    expected: "2-12".to_string(),
                });
            }
            if step_count > 12 {
                return Err(PolicyError::StepCountMismatch {
                    level: ExecutionLevel::MasterPlan,
                    bound: "many".to_string(),
                    count: step_count,
                    expected: "2-12".to_string(),
                });
            }
        }
    }

    // All levels require a Reply step
    if !has_reply {
        return Err(PolicyError::MissingReplyStep);
    }

    Ok(())
}

/// Check if program is overbuilt for the level
pub fn program_is_overbuilt(program: &Program, level: ExecutionLevel) -> bool {
    match level {
        ExecutionLevel::Action | ExecutionLevel::Task => program
            .steps
            .iter()
            .any(|s| matches!(s, Step::Plan { .. } | Step::MasterPlan { .. })),
        _ => false,
    }
}

/// Check if program is underbuilt for the level
pub fn program_is_underbuilt(program: &Program, level: ExecutionLevel) -> bool {
    match level {
        ExecutionLevel::Plan => !program.steps.iter().any(|s| matches!(s, Step::Plan { .. })),
        ExecutionLevel::MasterPlan => !program
            .steps
            .iter()
            .any(|s| matches!(s, Step::MasterPlan { .. })),
        _ => false,
    }
}

/// Validate that formula matches the execution level
///
/// Task 014: Formula should align with ladder-determined level.
/// This is a safety net - the main alignment happens in orchestration_planning.rs.
pub fn validate_formula_level(
    formula: &FormulaSelection,
    level: ExecutionLevel,
) -> Result<(), PolicyError> {
    let allowed_formulas = match level {
        ExecutionLevel::Action => vec!["reply_only", "execute_reply"],
        ExecutionLevel::Task => vec![
            "inspect_reply",
            "inspect_summarize_reply",
            "inspect_decide_reply",
            "inspect_edit_verify_reply",
        ],
        ExecutionLevel::Plan => vec!["plan_reply"],
        ExecutionLevel::MasterPlan => vec!["masterplan_reply"],
    };

    if !allowed_formulas
        .iter()
        .any(|f| formula.primary.eq_ignore_ascii_case(f))
    {
        return Err(PolicyError::FormulaNotAllowed {
            formula: formula.primary.clone(),
            level,
            allowed: allowed_formulas.join(", "),
        });
    }

    Ok(())
}
