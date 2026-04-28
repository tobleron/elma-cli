//! @efficiency-role: domain-logic
//!
//! Program Policy and Evaluation
//!
//! Task 044: Added execution level validation.

use crate::execution_ladder::ExecutionLevel;
use crate::*;

// Re-export level validation functions to maintain the same public API
pub use crate::program_policy_level::*;
use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum PolicyError {
    #[error("Program exceeds maximum step limit: {count} steps (max: {max}). This indicates a planning loop.")]
    MaxStepsExceeded { count: usize, max: usize },
    #[error("Program has {ratio}% duplicate steps (max: 50%). This indicates a planning loop.")]
    DuplicateStepRatio { ratio: usize },
    #[error("{level:?} request should not have {structure} structure")]
    InvalidLevelStructure {
        level: ExecutionLevel,
        structure: String,
    },
    #[error("{level:?} request has too {bound} steps: {count} (expected {expected})")]
    StepCountMismatch {
        level: ExecutionLevel,
        bound: String,
        count: usize,
        expected: String,
    },
    #[error("{level:?} level request must have explicit {step_type} step")]
    MissingRequiredStep {
        level: ExecutionLevel,
        step_type: String,
    },
    #[error("Program must have Reply step")]
    MissingReplyStep,
    #[error("Formula '{formula}' not allowed for {level:?} level (allowed: {allowed})")]
    FormulaNotAllowed {
        formula: String,
        level: ExecutionLevel,
        allowed: String,
    },
    #[error("Request requires workspace evidence but program has no shell/read/search step")]
    MissingEvidenceSteps,
}

/// Deterministic risk level based on step types in a program
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ProgramRisk {
    Low,
    Medium,
    High,
}

pub(crate) fn compute_program_risk(program: &Program) -> ProgramRisk {
    let mut max_risk = ProgramRisk::Low;
    for step in &program.steps {
        let step_risk = step_risk_level(step);
        match (step_risk, &max_risk) {
            (ProgramRisk::High, _) => {
                max_risk = ProgramRisk::High;
                break;
            }
            (ProgramRisk::Medium, ProgramRisk::Low) => {
                max_risk = ProgramRisk::Medium;
            }
            _ => {}
        }
    }
    max_risk
}

fn step_risk_level(step: &Step) -> ProgramRisk {
    match step {
        Step::Read { .. }
        | Step::Search { .. }
        | Step::Select { .. }
        | Step::Decide { .. }
        | Step::Plan { .. }
        | Step::MasterPlan { .. }
        | Step::Respond { .. }
        | Step::Explore { .. }
        | Step::Reply { .. }
        | Step::Summarize { .. } => ProgramRisk::Low,
        Step::Shell { cmd, .. } => {
            if command_is_readonly(cmd) {
                ProgramRisk::Low
            } else {
                ProgramRisk::Medium
            }
        }
        Step::Write { .. } => ProgramRisk::Medium,
        Step::Edit { .. } => ProgramRisk::High,
        Step::Delete { .. } => ProgramRisk::High,
    }
}

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

pub(crate) fn request_requires_workspace_evidence(
    route_decision: &RouteDecision,
    complexity: &ComplexityAssessment,
    formula: &FormulaSelection,
) -> bool {
    complexity.needs_evidence
        || formula.primary.starts_with("inspect_")
}

pub(crate) fn program_has_workspace_evidence_steps(program: &Program) -> bool {
    program.steps.iter().any(|step| {
        matches!(
            step,
            Step::Shell { .. } | Step::Read { .. } | Step::Search { .. }
        )
    })
}

pub(crate) fn step_results_have_workspace_evidence(step_results: &[StepResult]) -> bool {
    step_results.iter().any(|result| {
        matches!(result.kind.as_str(), "shell" | "read" | "search")
            && result.ok
            && (!result.summary.trim().is_empty()
                || result
                    .raw_output
                    .as_ref()
                    .is_some_and(|text| !text.trim().is_empty()))
    })
}

pub(crate) fn validate_evidence_requirements(
    program: &Program,
    route_decision: &RouteDecision,
    complexity: &ComplexityAssessment,
    formula: &FormulaSelection,
) -> Result<(), PolicyError> {
    if request_requires_workspace_evidence(route_decision, complexity, formula)
        && !program_has_workspace_evidence_steps(program)
    {
        return Err(PolicyError::MissingEvidenceSteps);
    }

    Ok(())
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
            Step::Edit { spec, .. } => {
                format!("edit:{}:{}", spec.operation.trim(), spec.path.trim())
            }
            Step::Reply { .. } => "reply".to_string(),
            Step::Respond { .. } => "respond".to_string(),
            Step::Explore { .. } => "explore".to_string(),
            Step::Write { path, .. } => format!("write:{}", path.trim()),
            Step::Delete { path, .. } => format!("delete:{}", path.trim()),
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
            Step::Read { .. } => {
                has_read = true;
            }
            Step::Search { .. } => {
                has_search = true;
            }
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
            Step::Respond { .. } => has_reply = true,
            Step::Explore { .. } => has_search = true,
            Step::Write { path, .. } => {
                has_edit = true;
                executable_in_tune = false;
                if path.trim().is_empty() {
                    shape_errors.push(format!("step {sid} missing write path"));
                }
            }
            Step::Delete { path, .. } => {
                has_edit = true;
                executable_in_tune = false;
                if path.trim().is_empty() {
                    shape_errors.push(format!("step {sid} missing delete path"));
                }
            }
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
            interrupt_behavior: InterruptBehavior::Graceful,
            ..Default::default()
        },
    }];
    true
}

/// Validate step flags consistency (Task 265)
pub(crate) fn validate_step_flags(program: &Program) -> Vec<String> {
    let mut errors = Vec::new();
    for step in &program.steps {
        let common = crate::step_common(step);
        // Check is_read_only vs is_destructive consistency
        if common.is_read_only && common.is_destructive {
            errors.push(format!(
                "Step {}: is_read_only and is_destructive cannot both be true",
                crate::step_id(step)
            ));
        }
        // Check is_concurrency_safe consistency
        if common.is_destructive && common.is_concurrency_safe {
            errors.push(format!(
                "Step {}: destructive steps should not be marked as concurrency-safe",
                crate::step_id(step)
            ));
        }
        // Check interrupt_behavior consistency
        if common.is_destructive && matches!(common.interrupt_behavior, InterruptBehavior::Complete)
        {
            errors.push(format!(
                "Step {}: destructive steps should not use Complete interrupt behavior",
                crate::step_id(step)
            ));
        }
    }
    errors
}
