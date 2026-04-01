//! @efficiency-role: domain-logic
//!
//! Program Policy and Evaluation
//!
//! Task 044: Added execution level validation.

use crate::*;
use crate::execution_ladder::ExecutionLevel;

pub(crate) fn program_safety_check(cmd: &str) -> bool {
    is_command_sane(cmd) && is_command_allowed(cmd)
}

pub(crate) fn is_command_allowed(cmd: &str) -> bool {
    let lower = cmd.to_lowercase();
    let tokens: Vec<String> = lower
        .split(|c: char| c.is_whitespace() || matches!(c, ';' | '|' | '&' | '(' | ')' | '<' | '>'))
        .filter(|s| !s.is_empty())
        .map(|s| s.rsplit('/').next().unwrap_or(s).to_string())
        .collect();

    let banned_cmds = [
        "curl", "wget", "ssh", "scp", "rsync", "nc", "netcat", "ping", "sudo", "shutdown", "reboot",
    ];

    if tokens.iter().any(|t| banned_cmds.contains(&t.as_str())) {
        return false;
    }

    for pair in tokens.windows(2) {
        if pair[0] == "rm" && (pair[1] == "-rf" || pair[1] == "-fr") {
            return false;
        }
    }

    true
}

pub(crate) fn command_is_readonly(cmd: &str) -> bool {
    let lower = cmd.to_lowercase();
    if lower.contains(" >")
        || lower.contains(">>")
        || lower.contains(">|")
        || lower.contains("tee ")
        || lower.contains("sed -i")
        || lower.contains("perl -pi")
    {
        return false;
    }

    let tokens: Vec<&str> = lower
        .split(|c: char| c.is_whitespace() || matches!(c, ';' | '|' | '&' | '(' | ')' | '<' | '>'))
        .filter(|s| !s.is_empty())
        .collect();
    if tokens.is_empty() {
        return false;
    }

    let first = tokens[0].rsplit('/').next().unwrap_or(tokens[0]);
    match first {
        "ls" | "pwd" | "cat" | "head" | "tail" | "rg" | "grep" | "find" | "awk" | "cut"
        | "sort" | "uniq" | "wc" | "basename" | "dirname" | "stat" | "tree" | "fd" | "jq"
        | "uname" | "whoami" | "tty" => return true,
        "sed" => return !tokens.iter().any(|t| *t == "-i"),
        "git" => {
            let sub = tokens.get(1).copied().unwrap_or("");
            return matches!(
                sub,
                "status" | "diff" | "log" | "show" | "branch" | "rev-parse"
            );
        }
        _ => {}
    }

    false
}

pub(crate) fn is_command_sane(cmd: &str) -> bool {
    let t = cmd.trim();
    if t.is_empty() {
        return false;
    }
    if t == "ls -" || t.ends_with(" ls -") || t.contains(" ls - ") {
        return false;
    }
    true
}

pub(crate) fn program_signature(program: &Program) -> String {
    program
        .steps
        .iter()
        .map(|step| match step {
            Step::Shell { cmd, .. } => format!("shell:{}", normalize_shell_cmd(cmd)),
            Step::Read { path, .. } => format!("read:{}", path.trim()),
            Step::Search { query, .. } => format!("search:{}", query.trim()),
            Step::Select { instructions, .. } => {
                format!("select:{}", instructions.trim())
            }
            Step::Plan { .. } => "plan".to_string(),
            Step::MasterPlan { .. } => "masterplan".to_string(),
            Step::Decide { .. } => "decide".to_string(),
            Step::Summarize { .. } => "summarize".to_string(),
            Step::Edit { spec, .. } => format!(
                "edit:{}:{}",
                spec.operation.trim(),
                spec.path.trim()
            ),
            Step::Reply { .. } => "reply".to_string(),
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

pub(crate) fn evaluate_program_for_scenario(
    program: &Program,
    scenario: &CalibrationScenario,
) -> ProgramEvaluation {
    let mut ids: HashMap<String, usize> = HashMap::new();
    let mut has_reply = false;
    let mut has_shell = false;
    let mut has_read = false;
    let mut has_search = false;
    let mut has_plan = false;
    let mut has_masterplan = false;
    let mut has_decide = false;
    let mut has_edit = false;
    let mut shape_errors = Vec::new();
    let mut policy_errors = Vec::new();
    let mut executable_in_tune = true;

    for step in &program.steps {
        let sid = step_id(step).to_string();
        *ids.entry(sid.clone()).or_insert(0usize) += 1;
        if step_purpose(step).trim().is_empty() {
            shape_errors.push(format!("step {sid} missing purpose"));
        }
        if step_success_condition(step).trim().is_empty() {
            shape_errors.push(format!("step {sid} missing success_condition"));
        }

        match step {
            Step::Shell { cmd, .. } => {
                has_shell = true;
                let normalized = normalize_shell_cmd(cmd);
                if !program_safety_check(&normalized) {
                    policy_errors.push(format!("shell step {sid} blocked by policy"));
                }
                if scenario.mode.as_deref() == Some("INSPECT") && !command_is_readonly(&normalized)
                {
                    policy_errors.push(format!("inspect shell step {sid} is not read-only"));
                }
                if !command_is_readonly(&normalized) {
                    executable_in_tune = false;
                }
            }
            Step::Read { .. } => { has_read = true; }
            Step::Search { .. } => { has_search = true; }
            Step::Select { .. } => {}
            Step::Plan { .. } => has_plan = true,
            Step::MasterPlan { .. } => has_masterplan = true,
            Step::Decide { .. } => has_decide = true,
            Step::Summarize { .. } => {}
            Step::Edit { spec, .. } => {
                has_edit = true;
                executable_in_tune = false;
                if spec.path.trim().is_empty() {
                    shape_errors.push(format!("step {sid} missing edit path"));
                }
                if spec.operation.trim().is_empty() {
                    shape_errors.push(format!("step {sid} missing edit operation"));
                }
            }
            Step::Reply { .. } => has_reply = true,
        }
    }

    for (id, count) in ids {
        if count > 1 {
            shape_errors.push(format!("duplicate step id {id}"));
        }
    }

    if program.steps.is_empty() {
        shape_errors.push("program has no steps".to_string());
    }
    if !has_reply {
        shape_errors.push("program has no reply step".to_string());
    }

    match scenario.route.as_str() {
        "CHAT" => {
            if has_shell || has_plan || has_masterplan || has_decide || has_edit {
                shape_errors.push("chat route should not execute workflow steps".to_string());
            }
        }
        "SHELL" => {
            if !(has_shell || has_edit) {
                shape_errors.push("shell route missing shell or edit step".to_string());
            }
        }
        "PLAN" => {
            if !has_plan {
                shape_errors.push("plan route missing plan step".to_string());
            }
        }
        "MASTERPLAN" => {
            if !has_masterplan {
                shape_errors.push("masterplan route missing masterplan step".to_string());
            }
        }
        "DECIDE" => {
            if !has_decide {
                shape_errors.push("decide route missing decide step".to_string());
            }
        }
        _ => {}
    }

    if scenario.speech_act == "CAPABILITY_CHECK"
        && (has_shell || has_plan || has_masterplan || has_decide || has_edit)
    {
        shape_errors.push("capability check should not execute or plan".to_string());
    }

    ProgramEvaluation {
        parsed: true,
        parse_error: String::new(),
        shape_ok: shape_errors.is_empty(),
        shape_reason: if shape_errors.is_empty() {
            "program structure matches scenario expectations".to_string()
        } else {
            shape_errors.join("; ")
        },
        policy_ok: policy_errors.is_empty(),
        policy_reason: if policy_errors.is_empty() {
            "program policy is acceptable".to_string()
        } else {
            policy_errors.join("; ")
        },
        executable_in_tune: executable_in_tune && policy_errors.is_empty(),
        signature: program_signature(program),
    }
}

pub(crate) fn capability_guard_threshold(route_decision: &RouteDecision) -> bool {
    route_decision
        .speech_act
        .choice
        .eq_ignore_ascii_case("CAPABILITY_CHECK")
        && probability_of(&route_decision.speech_act.distribution, "CAPABILITY_CHECK") >= 0.65
}

pub(crate) fn apply_capability_guard(
    program: &mut Program,
    route_decision: &RouteDecision,
    guards_enabled: bool,
) -> bool {
    if !guards_enabled {
        return false;
    }

    if !capability_guard_threshold(route_decision) {
        return false;
    }
    let has_non_reply = program
        .steps
        .iter()
        .any(|s| !matches!(s, Step::Reply { .. }));
    if !has_non_reply {
        return false;
    }

    let existing_reply = program.steps.iter().find_map(|s| match s {
        Step::Reply { instructions, .. } => Some(instructions.clone()),
        _ => None,
    });
    let instructions = existing_reply.unwrap_or_else(|| {
        "Answer the user's capability question in plain text. Do not execute commands. If helpful, say what Elma can do in this workspace and that you can do it if the user asks.".to_string()
    });
    program.steps = vec![Step::Reply {
        id: "r_cap".to_string(),
        instructions,
        common: StepCommon {
            purpose: "answer capability question without executing".to_string(),
            depends_on: Vec::new(),
            success_condition:
                "the user receives a plain-text capability answer with no command execution"
                    .to_string(),
            parent_id: None,
            depth: None,
            unit_type: None,
        },
    }];
    true
}

// ============================================================================
// Task 044: Execution Level Validation
// ============================================================================

/// Detect duplicate step ratio in a program
/// 
/// Task 014: Returns the ratio of duplicate steps (0.0 to 1.0).
/// High ratio indicates a planning loop.
fn detect_duplicate_step_ratio(program: &Program) -> f64 {
    if program.steps.len() < 2 {
        return 0.0;
    }

    // Group steps by (type, cmd/content, purpose) signature
    let mut step_signatures: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    
    for step in &program.steps {
        let signature = match step {
            Step::Shell { cmd, common, .. } => {
                format!("shell:{}:{}", cmd, common.purpose)
            }
            Step::Read { path, common, .. } => {
                format!("read:{}:{}", path, common.purpose)
            }
            Step::Search { query, paths, common, .. } => {
                format!("search:{}:{:?}", query, common.purpose)
            }
            Step::Select { instructions, common, .. } => {
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
            Step::Summarize { instructions, common, .. } => {
                format!("summarize:{}:{}", instructions, common.purpose)
            }
            Step::Edit { spec, common, .. } => {
                format!("edit:{}:{}:{}", spec.operation, spec.path, common.purpose)
            }
            Step::Reply { instructions, common, .. } => {
                format!("reply:{}:{}", instructions, common.purpose)
            }
        };
        
        *step_signatures.entry(signature).or_insert(0) += 1;
    }

    // Count duplicates (steps that appear more than once)
    let duplicate_count: usize = step_signatures.values()
        .filter(|&&count| count > 1)
        .sum();
    
    duplicate_count as f64 / program.steps.len() as f64
}

/// Check if a program matches the required execution level
///
/// Validates that the program is neither overbuilt nor underbuilt for the level.
/// Also enforces hard step limits to prevent plan collapse (40+ identical steps).
pub fn program_matches_level(program: &Program, required_level: ExecutionLevel) -> Result<(), String> {
    let has_plan = program.steps.iter().any(|s| matches!(s, Step::Plan { .. }));
    let has_masterplan = program.steps.iter().any(|s| matches!(s, Step::MasterPlan { .. }));
    let step_count = program.steps.len();
    let has_reply = program.steps.iter().any(|s| matches!(s, Step::Reply { .. }));

    // Task 014: Hard maximum step limit to prevent plan collapse
    // No program should exceed 12 steps - if it does, it's likely a loop
    const MAX_STEPS_ABSOLUTE: usize = 12;
    if step_count > MAX_STEPS_ABSOLUTE {
        return Err(format!(
            "Program exceeds maximum step limit: {} steps (max: {}). This indicates a planning loop.",
            step_count, MAX_STEPS_ABSOLUTE
        ));
    }

    // Task 014: Detect duplicate step loops
    // If >50% of steps are duplicates, reject as a loop
    if step_count >= 4 {
        let duplicate_ratio = detect_duplicate_step_ratio(program);
        if duplicate_ratio > 0.5 {
            return Err(format!(
                "Program has {}% duplicate steps (max: 50%). This indicates a planning loop.",
                (duplicate_ratio * 100.0) as usize
            ));
        }
    }

    match required_level {
        ExecutionLevel::AtomicOperation => {
            // Action level: should be 1-2 steps (primary action + reply)
            // Reject if has Plan/MasterPlan structure
            if has_plan {
                return Err(format!(
                    "Action-level request should not have Plan step ({} steps total)",
                    step_count
                ));
            }
            if has_masterplan {
                return Err(format!(
                    "Action-level request should not have MasterPlan step ({} steps total)",
                    step_count
                ));
            }
            // Allow 1-3 steps (action + optional evidence + reply)
            if step_count > 3 {
                return Err(format!(
                    "Action-level request has too many steps: {} (expected 1-3)",
                    step_count
                ));
            }
        }

        ExecutionLevel::DiscoveryTask => {
            // Task level: bounded outcome, 2-6 steps typical
            // Reject if has Plan/MasterPlan structure
            if has_plan {
                return Err(format!(
                    "Task-level request should not have Plan step ({} steps total)",
                    step_count
                ));
            }
            if has_masterplan {
                return Err(format!(
                    "Task-level request should not have MasterPlan step ({} steps total)",
                    step_count
                ));
            }
            // Allow 2-8 steps (evidence chain + transformation + reply)
            if step_count < 2 {
                return Err(format!(
                    "Task-level request has too few steps: {} (expected 2-8)",
                    step_count
                ));
            }
            if step_count > 8 {
                return Err(format!(
                    "Task-level request has too many steps: {} (expected 2-8)",
                    step_count
                ));
            }
        }

        ExecutionLevel::OperationalPlan => {
            // Plan level: must have explicit Plan step
            if !has_plan {
                return Err(
                    "Plan-level request must have explicit Plan step".to_string()
                );
            }
            // Should have reasonable structure (Plan + supporting steps + reply)
            // Task 014: Add upper bound to prevent explosion
            if step_count < 2 {
                return Err(format!(
                    "Plan-level request has too few steps: {} (expected 2-10)",
                    step_count
                ));
            }
            if step_count > 10 {
                return Err(format!(
                    "Plan-level request has too many steps: {} (expected 2-10)",
                    step_count
                ));
            }
        }

        ExecutionLevel::StrategicPlan => {
            // MasterPlan level: must have explicit MasterPlan step
            if !has_masterplan {
                return Err(
                    "MasterPlan-level request must have explicit MasterPlan step".to_string()
                );
            }
            // Should have strategic structure (MasterPlan + phases + reply)
            // Task 014: Add upper bound to prevent explosion
            if step_count < 2 {
                return Err(format!(
                    "MasterPlan-level request has too few steps: {} (expected 2-12)",
                    step_count
                ));
            }
            if step_count > 12 {
                return Err(format!(
                    "MasterPlan-level request has too many steps: {} (expected 2-12)",
                    step_count
                ));
            }
        }
    }

    // All levels require a Reply step
    if !has_reply {
        return Err("Program must have Reply step".to_string());
    }
    
    Ok(())
}

/// Check if program is overbuilt for the level
pub fn program_is_overbuilt(program: &Program, level: ExecutionLevel) -> bool {
    match level {
        ExecutionLevel::AtomicOperation | ExecutionLevel::DiscoveryTask => {
            program.steps.iter().any(|s| matches!(s, Step::Plan { .. } | Step::MasterPlan { .. }))
        }
        _ => false,
    }
}

/// Check if program is underbuilt for the level
pub fn program_is_underbuilt(program: &Program, level: ExecutionLevel) -> bool {
    match level {
        ExecutionLevel::OperationalPlan => !program.steps.iter().any(|s| matches!(s, Step::Plan { .. })),
        ExecutionLevel::StrategicPlan => !program.steps.iter().any(|s| matches!(s, Step::MasterPlan { .. })),
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
) -> Result<(), String> {
    let allowed_formulas = match level {
        ExecutionLevel::AtomicOperation => vec!["reply_only", "execute_reply"],
        ExecutionLevel::DiscoveryTask => vec!["inspect_reply", "inspect_summarize_reply", "inspect_decide_reply", "inspect_edit_verify_reply"],
        ExecutionLevel::OperationalPlan => vec!["plan_reply"],
        ExecutionLevel::StrategicPlan => vec!["masterplan_reply"],
    };

    if !allowed_formulas.iter().any(|f| formula.primary.eq_ignore_ascii_case(f)) {
        return Err(format!(
            "Formula '{}' not allowed for {:?} level (allowed: {})",
            formula.primary, level, allowed_formulas.join(", ")
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_program(steps: Vec<Step>) -> Program {
        Program {
            objective: "test".to_string(),
            steps,
        }
    }

    #[test]
    fn test_action_level_rejects_plan() {
        let program = make_program(vec![
            Step::Plan {
                id: "p1".to_string(),
                goal: "test".to_string(),
                common: StepCommon::default(),
            },
            Step::Reply {
                id: "r1".to_string(),
                instructions: "test".to_string(),
                common: StepCommon::default(),
            },
        ]);
        
        let result = program_matches_level(&program, ExecutionLevel::AtomicOperation);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Plan"));
    }

    #[test]
    fn test_action_level_accepts_simple_program() {
        let program = make_program(vec![
            Step::Shell {
                id: "s1".to_string(),
                cmd: "cargo test".to_string(),
                common: StepCommon::default(),
            },
            Step::Reply {
                id: "r1".to_string(),
                instructions: "test".to_string(),
                common: StepCommon::default(),
            },
        ]);
        
        let result = program_matches_level(&program, ExecutionLevel::AtomicOperation);
        assert!(result.is_ok());
    }

    #[test]
    fn test_task_level_rejects_plan() {
        let program = make_program(vec![
            Step::Read {
                id: "r1".to_string(),
                path: "file.txt".to_string(),
                common: StepCommon::default(),
            },
            Step::Plan {
                id: "p1".to_string(),
                goal: "test".to_string(),
                common: StepCommon::default(),
            },
            Step::Reply {
                id: "r2".to_string(),
                instructions: "test".to_string(),
                common: StepCommon::default(),
            },
        ]);
        
        let result = program_matches_level(&program, ExecutionLevel::DiscoveryTask);
        assert!(result.is_err());
    }

    #[test]
    fn test_plan_level_requires_plan_step() {
        let program = make_program(vec![
            Step::Shell {
                id: "s1".to_string(),
                cmd: "cargo test".to_string(),
                common: StepCommon::default(),
            },
            Step::Reply {
                id: "r1".to_string(),
                instructions: "test".to_string(),
                common: StepCommon::default(),
            },
        ]);
        
        let result = program_matches_level(&program, ExecutionLevel::OperationalPlan);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Plan"));
    }

    #[test]
    fn test_masterplan_level_requires_masterplan_step() {
        let program = make_program(vec![
            Step::Plan {
                id: "p1".to_string(),
                goal: "test".to_string(),
                common: StepCommon::default(),
            },
            Step::Reply {
                id: "r1".to_string(),
                instructions: "test".to_string(),
                common: StepCommon::default(),
            },
        ]);
        
        let result = program_matches_level(&program, ExecutionLevel::StrategicPlan);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("MasterPlan"));
    }

    #[test]
    fn test_program_is_overbuilt() {
        let program = make_program(vec![
            Step::Plan {
                id: "p1".to_string(),
                goal: "test".to_string(),
                common: StepCommon::default(),
            },
            Step::Reply {
                id: "r1".to_string(),
                instructions: "test".to_string(),
                common: StepCommon::default(),
            },
        ]);
        
        assert!(program_is_overbuilt(&program, ExecutionLevel::AtomicOperation));
        assert!(program_is_overbuilt(&program, ExecutionLevel::DiscoveryTask));
        assert!(!program_is_overbuilt(&program, ExecutionLevel::OperationalPlan));
    }

    #[test]
    fn test_program_is_underbuilt() {
        let program = make_program(vec![
            Step::Shell {
                id: "s1".to_string(),
                cmd: "cargo test".to_string(),
                common: StepCommon::default(),
            },
            Step::Reply {
                id: "r1".to_string(),
                instructions: "test".to_string(),
                common: StepCommon::default(),
            },
        ]);
        
        assert!(program_is_underbuilt(&program, ExecutionLevel::OperationalPlan));
        assert!(program_is_underbuilt(&program, ExecutionLevel::StrategicPlan));
        assert!(!program_is_underbuilt(&program, ExecutionLevel::AtomicOperation));
        assert!(!program_is_underbuilt(&program, ExecutionLevel::DiscoveryTask));
    }

    #[test]
    fn test_validate_formula_level_action() {
        let formula = FormulaSelection {
            primary: "reply_only".to_string(),
            alternatives: vec![],
            reason: "test".to_string(),
            memory_id: String::new(),
        };
        assert!(validate_formula_level(&formula, ExecutionLevel::AtomicOperation).is_ok());
        
        let formula = FormulaSelection {
            primary: "execute_reply".to_string(),
            alternatives: vec![],
            reason: "test".to_string(),
            memory_id: String::new(),
        };
        assert!(validate_formula_level(&formula, ExecutionLevel::AtomicOperation).is_ok());
        
        // Plan formula should fail for Action level
        let formula = FormulaSelection {
            primary: "plan_reply".to_string(),
            alternatives: vec![],
            reason: "test".to_string(),
            memory_id: String::new(),
        };
        assert!(validate_formula_level(&formula, ExecutionLevel::AtomicOperation).is_err());
    }

    #[test]
    fn test_validate_formula_level_task() {
        let formula = FormulaSelection {
            primary: "inspect_reply".to_string(),
            alternatives: vec![],
            reason: "test".to_string(),
            memory_id: String::new(),
        };
        assert!(validate_formula_level(&formula, ExecutionLevel::DiscoveryTask).is_ok());
        
        // Plan formula should fail for Task level
        let formula = FormulaSelection {
            primary: "plan_reply".to_string(),
            alternatives: vec![],
            reason: "test".to_string(),
            memory_id: String::new(),
        };
        assert!(validate_formula_level(&formula, ExecutionLevel::DiscoveryTask).is_err());
    }

    #[test]
    fn test_validate_formula_level_plan() {
        let formula = FormulaSelection {
            primary: "plan_reply".to_string(),
            alternatives: vec![],
            reason: "test".to_string(),
            memory_id: String::new(),
        };
        assert!(validate_formula_level(&formula, ExecutionLevel::OperationalPlan).is_ok());
        
        // Simple reply should fail for Plan level
        let formula = FormulaSelection {
            primary: "reply_only".to_string(),
            alternatives: vec![],
            reason: "test".to_string(),
            memory_id: String::new(),
        };
        assert!(validate_formula_level(&formula, ExecutionLevel::OperationalPlan).is_err());
    }

    #[test]
    fn test_detect_duplicate_step_ratio_no_duplicates() {
        let program = make_program(vec![
            Step::Shell {
                id: "s1".to_string(),
                cmd: "ls".to_string(),
                common: StepCommon { purpose: "list files".to_string(), ..StepCommon::default() },
            },
            Step::Shell {
                id: "s2".to_string(),
                cmd: "cat file.txt".to_string(),
                common: StepCommon { purpose: "read file".to_string(), ..StepCommon::default() },
            },
            Step::Reply {
                id: "r1".to_string(),
                instructions: "done".to_string(),
                common: StepCommon { purpose: "answer".to_string(), ..StepCommon::default() },
            },
        ]);
        
        let ratio = detect_duplicate_step_ratio(&program);
        assert!(ratio < 0.1);  // Should be 0 or very low
    }

    #[test]
    fn test_detect_duplicate_step_ratio_with_duplicates() {
        let program = make_program(vec![
            Step::Shell {
                id: "s1".to_string(),
                cmd: "grep fn".to_string(),
                common: StepCommon { purpose: "count functions".to_string(), ..StepCommon::default() },
            },
            Step::Shell {
                id: "s2".to_string(),
                cmd: "grep fn".to_string(),
                common: StepCommon { purpose: "count functions".to_string(), ..StepCommon::default() },
            },
            Step::Shell {
                id: "s3".to_string(),
                cmd: "grep fn".to_string(),
                common: StepCommon { purpose: "count functions".to_string(), ..StepCommon::default() },
            },
            Step::Shell {
                id: "s4".to_string(),
                cmd: "grep fn".to_string(),
                common: StepCommon { purpose: "count functions".to_string(), ..StepCommon::default() },
            },
        ]);
        
        let ratio = detect_duplicate_step_ratio(&program);
        assert!(ratio > 0.5);  // All 4 steps are duplicates
    }

    #[test]
    fn test_program_matches_level_rejects_excessive_steps() {
        // Create a program with 20 identical steps (simulating plan collapse)
        let mut steps = Vec::new();
        for i in 0..20 {
            steps.push(Step::Shell {
                id: format!("s{}", i),
                cmd: "grep fn".to_string(),
                common: StepCommon { purpose: "count functions".to_string(), ..StepCommon::default() },
            });
        }
        steps.push(Step::Reply {
            id: "r1".to_string(),
            instructions: "done".to_string(),
            common: StepCommon { purpose: "answer".to_string(), ..StepCommon::default() },
        });
        
        let program = Program {
            objective: "test".to_string(),
            steps,
        };
        
        // Should reject regardless of level due to absolute limit
        let result = program_matches_level(&program, ExecutionLevel::DiscoveryTask);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("maximum step limit"));
    }

    #[test]
    fn test_program_matches_level_rejects_duplicate_loop() {
        // Create a program with 50%+ duplicate steps
        let program = make_program(vec![
            Step::Shell {
                id: "s1".to_string(),
                cmd: "grep fn".to_string(),
                common: StepCommon { purpose: "count".to_string(), ..StepCommon::default() },
            },
            Step::Shell {
                id: "s2".to_string(),
                cmd: "grep fn".to_string(),
                common: StepCommon { purpose: "count".to_string(), ..StepCommon::default() },
            },
            Step::Shell {
                id: "s3".to_string(),
                cmd: "grep fn".to_string(),
                common: StepCommon { purpose: "count".to_string(), ..StepCommon::default() },
            },
            Step::Shell {
                id: "s4".to_string(),
                cmd: "grep fn".to_string(),
                common: StepCommon { purpose: "count".to_string(), ..StepCommon::default() },
            },
        ]);
        
        let result = program_matches_level(&program, ExecutionLevel::DiscoveryTask);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("duplicate steps"));
    }
}
