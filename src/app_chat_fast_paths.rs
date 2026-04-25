//! @efficiency-role: domain-logic
//! App Chat - Fast Path Detection and Direct Execution

use crate::app_chat_core::program_safety_check;
use crate::tool_discovery::command_exists;
use crate::*;

fn direct_shell_command_head(line: &str) -> Option<&str> {
    let head = line.split_whitespace().next()?.trim();
    if head.is_empty() {
        return None;
    }
    Some(head)
}

fn looks_like_literal_shell_command(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.ends_with('.') || trimmed.ends_with('?') || trimmed.ends_with('!') {
        return false;
    }
    let Some(head) = direct_shell_command_head(trimmed) else {
        return false;
    };
    if head
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_uppercase())
    {
        return false;
    }
    true
}

pub(crate) fn should_use_direct_shell_fast_path(
    line: &str,
    route_decision: &RouteDecision,
    workflow_plan: Option<&WorkflowPlannerOutput>,
    complexity: &ComplexityAssessment,
) -> bool {
    if !route_decision.route.eq_ignore_ascii_case("SHELL") {
        return false;
    }

    let complexity_allows_direct = complexity.complexity.eq_ignore_ascii_case("DIRECT")
        && complexity.risk.eq_ignore_ascii_case("LOW")
        && !complexity.needs_plan
        && !complexity.needs_decision;

    let workflow_allows_direct = workflow_plan.is_some_and(|plan| {
        (plan.complexity.trim().is_empty() || plan.complexity.eq_ignore_ascii_case("DIRECT"))
            && (plan.risk.trim().is_empty() || plan.risk.eq_ignore_ascii_case("LOW"))
    });

    if !complexity_allows_direct && !workflow_allows_direct {
        return false;
    }

    let Some(head) = direct_shell_command_head(line) else {
        return false;
    };

    if !looks_like_literal_shell_command(line) {
        return false;
    }

    command_exists(head) && program_safety_check(line) && command_is_readonly(line)
}

pub(crate) fn should_use_direct_reply_fast_path(
    line: &str,
    route_decision: &RouteDecision,
    complexity: &ComplexityAssessment,
    formula: &FormulaSelection,
) -> bool {
    let path_scoped_request = extract_first_path_from_user_text(line).is_some();
    if path_scoped_request {
        return false;
    }

    if route_decision.route.eq_ignore_ascii_case("CHAT")
        && formula.primary.eq_ignore_ascii_case("reply_only")
    {
        return true;
    }

    formula.primary.eq_ignore_ascii_case("reply_only")
        && complexity.complexity.eq_ignore_ascii_case("DIRECT")
        && complexity.risk.eq_ignore_ascii_case("LOW")
        && !complexity.needs_evidence
        && !complexity.needs_tools
        && !complexity.needs_decision
        && !complexity.needs_plan
}

pub(crate) fn build_direct_reply_program(line: &str) -> Program {
    Program {
        objective: line.to_string(),
        steps: vec![Step::Reply {
            id: "r1".to_string(),
            instructions: "Answer the user's message directly in plain terminal text. If the user asks who you are or what you do, reply in first person, start with `I'm Elma,`, and describe yourself as the local autonomous CLI agent for this workspace. Do not call yourself an AI language model. Use known runtime context facts if relevant. Do not invent configuration, workspace, or tool details."
                .to_string(),
            common: StepCommon {
                purpose: "direct grounded reply".to_string(),
                depends_on: Vec::new(),
                success_condition: "the user receives a direct truthful answer".to_string(),
                parent_id: None,
                depth: None,
                unit_type: None,
                is_read_only: true,
                is_destructive: false,
                is_concurrency_safe: true,
                        interrupt_behavior: InterruptBehavior::Graceful,
            },
        }],
    }
}

pub(crate) fn build_direct_shell_program(line: &str) -> Program {
    Program {
        objective: line.to_string(),
        steps: vec![
            Step::Shell {
                id: "s1".to_string(),
                cmd: line.to_string(),
                common: StepCommon {
                    purpose: "execute the requested shell command directly".to_string(),
                    depends_on: Vec::new(),
                    success_condition: "the requested command completes".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                    is_read_only: false,
                    is_destructive: true,
                    is_concurrency_safe: false,
                        interrupt_behavior: InterruptBehavior::Graceful,
                },
            },
            Step::Reply {
                id: "r1".to_string(),
                instructions:
                    "Report the command result clearly. If the output is short, show the relevant raw output."
                        .to_string(),
                common: StepCommon {
                    purpose: "present the shell result to the user".to_string(),
                    depends_on: vec!["s1".to_string()],
                    success_condition: "the user receives the command result".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                    is_read_only: true,
                    is_destructive: false,
                    is_concurrency_safe: true,
                        interrupt_behavior: InterruptBehavior::Graceful,
                },
            },
        ],
    }
}
