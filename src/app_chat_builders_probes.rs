//! @efficiency-role: util-pure
//!
//! App Chat - Probe Program Builder Helpers

use crate::*;

pub(crate) fn leaf_common(purpose: &str, success_condition: &str) -> StepCommon {
    StepCommon {
        purpose: purpose.to_string(),
        depends_on: Vec::new(),
        success_condition: success_condition.to_string(),
        parent_id: None,
        depth: None,
        unit_type: None,
    }
}

pub(crate) fn dep_common(purpose: &str, deps: &[&str], success_condition: &str) -> StepCommon {
    StepCommon {
        purpose: purpose.to_string(),
        depends_on: deps.iter().map(|s| s.to_string()).collect(),
        success_condition: success_condition.to_string(),
        parent_id: None,
        depth: None,
        unit_type: None,
    }
}

pub(crate) fn shell_step(id: &str, cmd: &str, purpose: &str, success_condition: &str) -> Step {
    Step::Shell {
        id: id.to_string(),
        cmd: cmd.to_string(),
        common: leaf_common(purpose, success_condition),
    }
}

pub(crate) fn shell_step_with_deps(
    id: &str,
    cmd: &str,
    purpose: &str,
    deps: &[&str],
    success_condition: &str,
) -> Step {
    Step::Shell {
        id: id.to_string(),
        cmd: cmd.to_string(),
        common: dep_common(purpose, deps, success_condition),
    }
}

pub(crate) fn reply_step(
    id: &str,
    instructions: &str,
    deps: &[&str],
    purpose: &str,
    success_condition: &str,
) -> Step {
    Step::Reply {
        id: id.to_string(),
        instructions: instructions.to_string(),
        common: dep_common(purpose, deps, success_condition),
    }
}

pub(crate) fn select_step(
    id: &str,
    instructions: &str,
    deps: &[&str],
    purpose: &str,
    success_condition: &str,
) -> Step {
    Step::Select {
        id: id.to_string(),
        instructions: instructions.to_string(),
        common: dep_common(purpose, deps, success_condition),
    }
}

pub(crate) fn select_step_with_unit(
    id: &str,
    instructions: &str,
    deps: &[&str],
    purpose: &str,
    success_condition: &str,
    unit: &str,
) -> Step {
    Step::Select {
        id: id.to_string(),
        instructions: instructions.to_string(),
        common: StepCommon {
            purpose: purpose.to_string(),
            depends_on: deps.iter().map(|s| s.to_string()).collect(),
            success_condition: success_condition.to_string(),
            parent_id: None,
            depth: None,
            unit_type: Some(unit.to_string()),
        },
    }
}

pub(crate) fn summarize_step(
    id: &str,
    deps: &[&str],
    purpose: &str,
    success_condition: &str,
    instructions: &str,
) -> Step {
    Step::Summarize {
        id: id.to_string(),
        text: String::new(),
        instructions: instructions.to_string(),
        common: dep_common(purpose, deps, success_condition),
    }
}

pub(crate) fn read_step(id: &str, path: &str, purpose: &str, success_condition: &str) -> Step {
    Step::Read {
        id: id.to_string(),
        path: path.to_string(),
        common: leaf_common(purpose, success_condition),
    }
}
