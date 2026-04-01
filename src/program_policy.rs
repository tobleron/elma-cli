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

/// Check if a program matches the required execution level
///
/// Validates that the program is neither overbuilt nor underbuilt for the level.
pub fn program_matches_level(program: &Program, required_level: ExecutionLevel) -> Result<(), String> {
    let has_plan = program.steps.iter().any(|s| matches!(s, Step::Plan { .. }));
    let has_masterplan = program.steps.iter().any(|s| matches!(s, Step::MasterPlan { .. }));
    let step_count = program.steps.len();
    let has_reply = program.steps.iter().any(|s| matches!(s, Step::Reply { .. }));
    
    match required_level {
        ExecutionLevel::Action => {
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
        
        ExecutionLevel::Task => {
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
        
        ExecutionLevel::Plan => {
            // Plan level: must have explicit Plan step
            if !has_plan {
                return Err(
                    "Plan-level request must have explicit Plan step".to_string()
                );
            }
            // Should have reasonable structure (Plan + supporting steps + reply)
            if step_count < 2 {
                return Err(format!(
                    "Plan-level request has too few steps: {} (expected 2+)",
                    step_count
                ));
            }
        }
        
        ExecutionLevel::MasterPlan => {
            // MasterPlan level: must have explicit MasterPlan step
            if !has_masterplan {
                return Err(
                    "MasterPlan-level request must have explicit MasterPlan step".to_string()
                );
            }
            // Should have strategic structure (MasterPlan + phases + reply)
            if step_count < 2 {
                return Err(format!(
                    "MasterPlan-level request has too few steps: {} (expected 2+)",
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
        ExecutionLevel::Action | ExecutionLevel::Task => {
            program.steps.iter().any(|s| matches!(s, Step::Plan { .. } | Step::MasterPlan { .. }))
        }
        _ => false,
    }
}

/// Check if program is underbuilt for the level
pub fn program_is_underbuilt(program: &Program, level: ExecutionLevel) -> bool {
    match level {
        ExecutionLevel::Plan => !program.steps.iter().any(|s| matches!(s, Step::Plan { .. })),
        ExecutionLevel::MasterPlan => !program.steps.iter().any(|s| matches!(s, Step::MasterPlan { .. })),
        _ => false,
    }
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
        
        let result = program_matches_level(&program, ExecutionLevel::Action);
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
        
        let result = program_matches_level(&program, ExecutionLevel::Action);
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
        
        let result = program_matches_level(&program, ExecutionLevel::Task);
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
        
        let result = program_matches_level(&program, ExecutionLevel::Plan);
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
        
        let result = program_matches_level(&program, ExecutionLevel::MasterPlan);
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
        
        assert!(program_is_overbuilt(&program, ExecutionLevel::Action));
        assert!(program_is_overbuilt(&program, ExecutionLevel::Task));
        assert!(!program_is_overbuilt(&program, ExecutionLevel::Plan));
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
        
        assert!(program_is_underbuilt(&program, ExecutionLevel::Plan));
        assert!(program_is_underbuilt(&program, ExecutionLevel::MasterPlan));
        assert!(!program_is_underbuilt(&program, ExecutionLevel::Action));
        assert!(!program_is_underbuilt(&program, ExecutionLevel::Task));
    }
}
