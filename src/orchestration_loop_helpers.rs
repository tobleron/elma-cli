//! @efficiency-role: util-pure
//!
//! Orchestration Loop - Helper Functions

use crate::*;

pub(crate) fn merged_program_from_history(plan: &AgentPlan) -> Program {
    let mut steps = Vec::new();
    for program in &plan.program_history {
        steps.extend(program.steps.clone());
    }
    Program {
        objective: plan.objective.clone(),
        steps,
    }
}

pub(crate) fn next_program_is_stale(plan: &AgentPlan, next_program: &Program) -> bool {
    program_signature(&plan.current_program) == program_signature(next_program)
}

pub(crate) fn program_has_shell_or_edit(program: &Program) -> bool {
    program.steps.iter().any(|step| matches!(step, Step::Shell { .. } | Step::Edit { .. }))
}

pub(crate) fn step_results_have_shell_or_edit(step_results: &[StepResult]) -> bool {
    step_results
        .iter()
        .any(|result| matches!(result.kind.as_str(), "shell" | "edit"))
}
