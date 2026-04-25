//! @efficiency-role: ignored

#[cfg(test)]
mod tests {
    use crate::types_core::{Program, Step, StepCommon};
    use crate::InterruptBehavior;

    #[test]
    fn test_program_equality() {
        let p1 = Program {
            objective: "test".to_string(),
            steps: vec![Step::Shell {
                id: "s1".to_string(),
                cmd: "ls".to_string(),
                common: StepCommon {
                    purpose: "list".to_string(),
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
            }],
        };
        let p2 = Program {
            objective: "test".to_string(),
            steps: vec![Step::Shell {
                id: "s1".to_string(),
                cmd: "ls".to_string(),
                common: StepCommon {
                    purpose: "list".to_string(),
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
            }],
        };
        let p3 = Program {
            objective: "test".to_string(),
            steps: vec![Step::Shell {
                id: "s1".to_string(),
                cmd: "ls -la".to_string(),
                common: StepCommon {
                    purpose: "list".to_string(),
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
            }],
        };

    assert_eq!(p1, p2);
    assert_ne!(p1, p3);
    }
}
