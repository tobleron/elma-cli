//! @efficiency-role: ignored
//!
//! Program Policy Tests
//!
//! Tests for core policy and level validation functions.

#[cfg(test)]
mod tests {
    use crate::program_policy::*;
    use crate::program_policy_level::*;
    use crate::*;

    fn test_probability_decision(choice: &str) -> ProbabilityDecision {
        ProbabilityDecision {
            choice: choice.to_string(),
            source: "test".to_string(),
            distribution: vec![(choice.to_string(), 1.0)],
            margin: 1.0,
            entropy: 0.0,
        }
    }

    fn test_route_decision(route: &str) -> RouteDecision {
        RouteDecision {
            route: route.to_string(),
            source: "test".to_string(),
            distribution: vec![(route.to_string(), 1.0)],
            margin: 1.0,
            entropy: 0.0,
            speech_act: test_probability_decision("INSTRUCT"),
            workflow: test_probability_decision("WORKFLOW"),
            mode: test_probability_decision("DECIDE"),
            evidence_required: false,
        }
    }

    fn make_program(steps: Vec<Step>) -> Program {
        Program {
            objective: "test".to_string(),
            steps,
        }
    }

    // ========================================================================
    // Level validation tests
    // ========================================================================

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
        assert!(result.unwrap_err().to_string().contains("Plan"));
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
                path: Some("file.txt".to_string()),
                paths: None,
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
        assert!(result.unwrap_err().to_string().contains("Plan"));
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
        assert!(result.unwrap_err().to_string().contains("MasterPlan"));
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

    #[test]
    fn test_validate_formula_level_action() {
        let formula = FormulaSelection {
            primary: "reply_only".to_string(),
            alternatives: vec![],
            reason: "test".to_string(),
            memory_id: String::new(),
        };
        assert!(validate_formula_level(&formula, ExecutionLevel::Action).is_ok());

        let formula = FormulaSelection {
            primary: "execute_reply".to_string(),
            alternatives: vec![],
            reason: "test".to_string(),
            memory_id: String::new(),
        };
        assert!(validate_formula_level(&formula, ExecutionLevel::Action).is_ok());

        // Plan formula should fail for Action level
        let formula = FormulaSelection {
            primary: "plan_reply".to_string(),
            alternatives: vec![],
            reason: "test".to_string(),
            memory_id: String::new(),
        };
        assert!(validate_formula_level(&formula, ExecutionLevel::Action).is_err());
    }

    #[test]
    fn test_validate_formula_level_task() {
        let formula = FormulaSelection {
            primary: "inspect_reply".to_string(),
            alternatives: vec![],
            reason: "test".to_string(),
            memory_id: String::new(),
        };
        assert!(validate_formula_level(&formula, ExecutionLevel::Task).is_ok());

        // Plan formula should fail for Task level
        let formula = FormulaSelection {
            primary: "plan_reply".to_string(),
            alternatives: vec![],
            reason: "test".to_string(),
            memory_id: String::new(),
        };
        assert!(validate_formula_level(&formula, ExecutionLevel::Task).is_err());
    }

    #[test]
    fn test_validate_formula_level_plan() {
        let formula = FormulaSelection {
            primary: "plan_reply".to_string(),
            alternatives: vec![],
            reason: "test".to_string(),
            memory_id: String::new(),
        };
        assert!(validate_formula_level(&formula, ExecutionLevel::Plan).is_ok());

        // Simple reply should fail for Plan level
        let formula = FormulaSelection {
            primary: "reply_only".to_string(),
            alternatives: vec![],
            reason: "test".to_string(),
            memory_id: String::new(),
        };
        assert!(validate_formula_level(&formula, ExecutionLevel::Plan).is_err());
    }

    #[test]
    fn test_detect_duplicate_step_ratio_no_duplicates() {
        let program = make_program(vec![
            Step::Shell {
                id: "s1".to_string(),
                cmd: "ls".to_string(),
                common: StepCommon {
                    purpose: "list files".to_string(),
                    depends_on: Vec::new(),
                    success_condition: "ok".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                    interrupt_behavior: InterruptBehavior::Graceful,
                    ..Default::default()
                },
            },
            Step::Shell {
                id: "s2".to_string(),
                cmd: "cat file.txt".to_string(),
                common: StepCommon {
                    purpose: "read file".to_string(),
                    depends_on: Vec::new(),
                    success_condition: "ok".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                    interrupt_behavior: InterruptBehavior::Graceful,
                    ..Default::default()
                },
            },
            Step::Reply {
                id: "r1".to_string(),
                instructions: "done".to_string(),
                common: StepCommon {
                    purpose: "answer".to_string(),
                    depends_on: Vec::new(),
                    success_condition: "ok".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                    interrupt_behavior: InterruptBehavior::Graceful,
                    ..Default::default()
                },
            },
        ]);

        let ratio = detect_duplicate_step_ratio(&program);
        assert!(ratio < 0.1); // Should be 0 or very low
    }

    #[test]
    fn test_detect_duplicate_step_ratio_with_duplicates() {
        let program = make_program(vec![
            Step::Shell {
                id: "s1".to_string(),
                cmd: "grep fn".to_string(),
                common: StepCommon {
                    purpose: "count functions".to_string(),
                    depends_on: Vec::new(),
                    success_condition: "ok".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                    interrupt_behavior: InterruptBehavior::Graceful,
                    ..Default::default()
                },
            },
            Step::Shell {
                id: "s2".to_string(),
                cmd: "grep fn".to_string(),
                common: StepCommon {
                    purpose: "count functions".to_string(),
                    depends_on: Vec::new(),
                    success_condition: "ok".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                    interrupt_behavior: InterruptBehavior::Graceful,
                    ..Default::default()
                },
            },
            Step::Shell {
                id: "s3".to_string(),
                cmd: "grep fn".to_string(),
                common: StepCommon {
                    purpose: "count functions".to_string(),
                    depends_on: Vec::new(),
                    success_condition: "ok".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                    interrupt_behavior: InterruptBehavior::Graceful,
                    ..Default::default()
                },
            },
            Step::Shell {
                id: "s4".to_string(),
                cmd: "grep fn".to_string(),
                common: StepCommon {
                    purpose: "count functions".to_string(),
                    depends_on: Vec::new(),
                    success_condition: "ok".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                    interrupt_behavior: InterruptBehavior::Graceful,
                    ..Default::default()
                },
            },
        ]);

        let ratio = detect_duplicate_step_ratio(&program);
        assert!(ratio > 0.5); // All 4 steps are duplicates
    }

    #[test]
    fn test_program_matches_level_rejects_excessive_steps() {
        // Create a program with 20 identical steps (simulating plan collapse)
        let mut steps = Vec::new();
        for i in 0..20 {
            steps.push(Step::Shell {
                id: format!("s{}", i),
                cmd: "grep fn".to_string(),
                common: StepCommon {
                    purpose: "count functions".to_string(),
                    depends_on: Vec::new(),
                    success_condition: "ok".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                    interrupt_behavior: InterruptBehavior::Graceful,
                    ..Default::default()
                },
            });
        }
        steps.push(Step::Reply {
            id: "r1".to_string(),
            instructions: "done".to_string(),
            common: StepCommon {
                purpose: "answer".to_string(),
                depends_on: Vec::new(),
                success_condition: "ok".to_string(),
                parent_id: None,
                depth: None,
                unit_type: None,
                interrupt_behavior: InterruptBehavior::Graceful,
                ..Default::default()
            },
        });

        let program = Program {
            objective: "test".to_string(),
            steps,
        };

        // Should reject regardless of level due to absolute limit
        let result = program_matches_level(&program, ExecutionLevel::Task);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("maximum step limit"));
    }

    #[test]
    fn test_program_matches_level_rejects_duplicate_loop() {
        // Create a program with 50%+ duplicate steps
        let program = make_program(vec![
            Step::Shell {
                id: "s1".to_string(),
                cmd: "grep fn".to_string(),
                common: StepCommon {
                    purpose: "count".to_string(),
                    depends_on: Vec::new(),
                    success_condition: "ok".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                    interrupt_behavior: InterruptBehavior::Graceful,
                    ..Default::default()
                },
            },
            Step::Shell {
                id: "s2".to_string(),
                cmd: "grep fn".to_string(),
                common: StepCommon {
                    purpose: "count".to_string(),
                    depends_on: Vec::new(),
                    success_condition: "ok".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                    interrupt_behavior: InterruptBehavior::Graceful,
                    ..Default::default()
                },
            },
            Step::Shell {
                id: "s3".to_string(),
                cmd: "grep fn".to_string(),
                common: StepCommon {
                    purpose: "count".to_string(),
                    depends_on: Vec::new(),
                    success_condition: "ok".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                    interrupt_behavior: InterruptBehavior::Graceful,
                    ..Default::default()
                },
            },
            Step::Shell {
                id: "s4".to_string(),
                cmd: "grep fn".to_string(),
                common: StepCommon {
                    purpose: "count".to_string(),
                    depends_on: Vec::new(),
                    success_condition: "ok".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                    ..StepCommon::default()
                },
            },
        ]);

        let result = program_matches_level(&program, ExecutionLevel::Task);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("duplicate steps"));
    }

    // ========================================================================
    // Core policy tests
    // ========================================================================

    #[test]
    fn test_validate_evidence_requirements_rejects_evidence_free_inspect_decide_program() {
        let route = test_route_decision("DECIDE");
        let complexity = ComplexityAssessment {
            needs_evidence: true,
            ..ComplexityAssessment::default()
        };
        let formula = FormulaSelection {
            primary: "inspect_decide_reply".to_string(),
            ..FormulaSelection::default()
        };
        let program = make_program(vec![
            Step::Decide {
                id: "d1".to_string(),
                prompt: "decide".to_string(),
                common: StepCommon {
                    purpose: "decide".to_string(),
                    depends_on: Vec::new(),
                    success_condition: "ok".to_string(),
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
                instructions: "answer".to_string(),
                common: StepCommon {
                    purpose: "answer".to_string(),
                    depends_on: Vec::new(),
                    success_condition: "ok".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                    is_read_only: true,
                    is_destructive: false,
                    is_concurrency_safe: true,
                    interrupt_behavior: InterruptBehavior::Graceful,
                },
            },
        ]);

        let result = validate_evidence_requirements(&program, &route, &complexity, &formula);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("requires workspace evidence"));
    }

    #[test]
    fn test_validate_evidence_requirements_accepts_grounded_inspect_decide_program() {
        let route = test_route_decision("DECIDE");
        let complexity = ComplexityAssessment {
            needs_evidence: true,
            ..ComplexityAssessment::default()
        };
        let formula = FormulaSelection {
            primary: "inspect_decide_reply".to_string(),
            ..FormulaSelection::default()
        };
        let program = make_program(vec![
            Step::Shell {
                id: "s1".to_string(),
                cmd: "rg --files .".to_string(),
                common: StepCommon {
                    purpose: "search".to_string(),
                    depends_on: Vec::new(),
                    success_condition: "ok".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                    is_read_only: false,
                    is_destructive: true,
                    is_concurrency_safe: false,
                    interrupt_behavior: InterruptBehavior::Graceful,
                    ..Default::default()
                },
            },
            Step::Decide {
                id: "d1".to_string(),
                prompt: "decide".to_string(),
                common: StepCommon {
                    purpose: "decide".to_string(),
                    depends_on: Vec::new(),
                    success_condition: "ok".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                    is_read_only: false,
                    is_destructive: true,
                    is_concurrency_safe: false,
                    interrupt_behavior: InterruptBehavior::Graceful,
                    ..Default::default()
                },
            },
            Step::Reply {
                id: "r1".to_string(),
                instructions: "answer".to_string(),
                common: StepCommon {
                    purpose: "answer".to_string(),
                    depends_on: Vec::new(),
                    success_condition: "ok".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                    is_read_only: true,
                    is_destructive: false,
                    is_concurrency_safe: true,
                    interrupt_behavior: InterruptBehavior::Graceful,
                    ..Default::default()
                },
            },
        ]);

        assert!(validate_evidence_requirements(&program, &route, &complexity, &formula).is_ok());
    }

    #[test]
    fn test_step_results_have_workspace_evidence_detects_grounded_read() {
        let step_results = vec![StepResult {
            id: "r1".to_string(),
            kind: "read".to_string(),
            ok: true,
            summary: "read README".to_string(),
            raw_output: Some("hello".to_string()),
            ..StepResult::default()
        }];

        assert!(step_results_have_workspace_evidence(&step_results));
    }
}
