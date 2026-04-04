//! @efficiency-role: domain-logic
//!
//! Evidence-based verification helpers for step outcomes.

use crate::*;

pub(crate) fn has_downstream_dependents(program: &Program, step_id_value: &str) -> bool {
    program
        .steps
        .iter()
        .any(|step| step_depends_on(step).iter().any(|dep| dep == step_id_value))
}

pub(crate) fn is_intermediate_shell_evidence_step(program: &Program, result: &StepResult) -> bool {
    result.kind == "shell"
        && result.exit_code == Some(0)
        && result
            .raw_output
            .as_ref()
            .is_some_and(|text| !text.trim().is_empty())
        && has_downstream_dependents(program, &result.id)
}

pub(crate) fn has_verified_downstream_evidence(
    program: &Program,
    step_results: &[StepResult],
    result_id: &str,
) -> bool {
    let dependent_ids: Vec<String> = program
        .steps
        .iter()
        .filter(|step| step_depends_on(step).iter().any(|dep| dep == result_id))
        .map(|step| step_id(step).to_string())
        .collect();

    if dependent_ids.is_empty() {
        return false;
    }

    step_results.iter().any(|downstream| {
        dependent_ids.iter().any(|id| id == &downstream.id)
            && downstream.ok
            && matches!(downstream.kind.as_str(), "read" | "search" | "shell")
            && downstream
                .raw_output
                .as_ref()
                .is_some_and(|text| !text.trim().is_empty())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intermediate_shell_evidence_step_is_detected() {
        let program = Program {
            objective: "test".to_string(),
            steps: vec![
                Step::Shell {
                    id: "s1".to_string(),
                    cmd: "ls".to_string(),
                    common: StepCommon::default(),
                },
                Step::Select {
                    id: "sel1".to_string(),
                    instructions: "pick one".to_string(),
                    common: StepCommon {
                        depends_on: vec!["s1".to_string()],
                        ..StepCommon::default()
                    },
                },
            ],
        };

        let result = StepResult {
            id: "s1".to_string(),
            kind: "shell".to_string(),
            ok: true,
            raw_output: Some("main.go\ncmd/root.go".to_string()),
            exit_code: Some(0),
            ..StepResult::default()
        };

        assert!(is_intermediate_shell_evidence_step(&program, &result));
    }

    #[test]
    fn standalone_shell_step_is_not_treated_as_intermediate_evidence() {
        let program = Program {
            objective: "test".to_string(),
            steps: vec![Step::Shell {
                id: "s1".to_string(),
                cmd: "ls".to_string(),
                common: StepCommon::default(),
            }],
        };

        let result = StepResult {
            id: "s1".to_string(),
            kind: "shell".to_string(),
            ok: true,
            raw_output: Some("main.go".to_string()),
            exit_code: Some(0),
            ..StepResult::default()
        };

        assert!(!is_intermediate_shell_evidence_step(&program, &result));
    }

    #[test]
    fn edit_with_downstream_read_verification_is_detected() {
        let program = Program {
            objective: "test".to_string(),
            steps: vec![
                Step::Edit {
                    id: "e1".to_string(),
                    spec: EditSpec {
                        path: "README.md".to_string(),
                        operation: "append_text".to_string(),
                        content: "hello".to_string(),
                        ..EditSpec::default()
                    },
                    common: StepCommon::default(),
                },
                Step::Read {
                    id: "r1".to_string(),
                    path: "README.md".to_string(),
                    common: StepCommon {
                        depends_on: vec!["e1".to_string()],
                        ..StepCommon::default()
                    },
                },
            ],
        };

        let results = vec![
            StepResult {
                id: "e1".to_string(),
                kind: "edit".to_string(),
                ok: true,
                ..StepResult::default()
            },
            StepResult {
                id: "r1".to_string(),
                kind: "read".to_string(),
                ok: true,
                raw_output: Some("## Heading\nThis sandbox was exercised.".to_string()),
                exit_code: Some(0),
                ..StepResult::default()
            },
        ];

        assert!(has_verified_downstream_evidence(
            &program,
            &results,
            &results[0].id
        ));
    }

    #[test]
    fn edit_without_grounded_downstream_evidence_is_not_detected() {
        let program = Program {
            objective: "test".to_string(),
            steps: vec![
                Step::Edit {
                    id: "e1".to_string(),
                    spec: EditSpec {
                        path: "README.md".to_string(),
                        operation: "append_text".to_string(),
                        content: "hello".to_string(),
                        ..EditSpec::default()
                    },
                    common: StepCommon::default(),
                },
                Step::Read {
                    id: "r1".to_string(),
                    path: "README.md".to_string(),
                    common: StepCommon {
                        depends_on: vec!["e1".to_string()],
                        ..StepCommon::default()
                    },
                },
            ],
        };

        let results = vec![
            StepResult {
                id: "e1".to_string(),
                kind: "edit".to_string(),
                ok: true,
                ..StepResult::default()
            },
            StepResult {
                id: "r1".to_string(),
                kind: "read".to_string(),
                ok: true,
                raw_output: None,
                exit_code: Some(0),
                ..StepResult::default()
            },
        ];

        assert!(!has_verified_downstream_evidence(
            &program,
            &results,
            &results[0].id
        ));
    }
}
