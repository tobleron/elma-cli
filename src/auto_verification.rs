//! @efficiency-role: domain-logic
//!
//! Auto lint, test, and verification planner (Task 675).
//! Generates a verification plan from a set of changed files.

use std::path::Path;

#[derive(Debug, Clone)]
pub(crate) struct VerificationStep {
    pub(crate) tool: String,
    pub(crate) target: String,
    pub(crate) timeout_secs: u64,
    pub(crate) critical: bool,
}

pub(crate) struct VerificationPlanner;

impl VerificationPlanner {
    pub(crate) fn plan(changed_files: &[std::path::PathBuf]) -> Vec<VerificationStep> {
        let mut steps = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for file in changed_files {
            for step in Self::steps_for_file(file) {
                let key = format!("{}:{}", step.tool, step.target);
                if seen.insert(key) {
                    steps.push(step);
                }
            }
        }
        steps
    }

    pub(crate) fn steps_for_file(path: &Path) -> Vec<VerificationStep> {
        match path.extension().and_then(|e| e.to_str()) {
            Some("rs") => {
                vec![
                    VerificationStep {
                        tool: "cargo".into(),
                        target: "check".into(),
                        timeout_secs: 120,
                        critical: true,
                    },
                    VerificationStep {
                        tool: "cargo".into(),
                        target: "test".into(),
                        timeout_secs: 300,
                        critical: true,
                    },
                ]
            }
            Some("md") => {
                vec![VerificationStep {
                    tool: "markdown".into(),
                    target: path.to_string_lossy().to_string(),
                    timeout_secs: 30,
                    critical: false,
                }]
            }
            Some("sh") => {
                vec![VerificationStep {
                    tool: "shellcheck".into(),
                    target: path.to_string_lossy().to_string(),
                    timeout_secs: 30,
                    critical: true,
                }]
            }
            Some("toml") => {
                vec![VerificationStep {
                    tool: "cargo".into(),
                    target: "verify".into(),
                    timeout_secs: 60,
                    critical: true,
                }]
            }
            _ => Vec::new(),
        }
    }

    pub(crate) fn should_skip(step: &VerificationStep) -> bool {
        !step.critical
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_plan_rs_file_check_and_test() {
        let files = vec![PathBuf::from("src/main.rs")];
        let steps = VerificationPlanner::plan(&files);
        assert_eq!(steps.len(), 2);
        assert!(steps
            .iter()
            .any(|s| s.tool == "cargo" && s.target == "check"));
        assert!(steps
            .iter()
            .any(|s| s.tool == "cargo" && s.target == "test"));
    }

    #[test]
    fn test_plan_sh_file_shellcheck() {
        let files = vec![PathBuf::from("scripts/deploy.sh")];
        let steps = VerificationPlanner::plan(&files);
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].tool, "shellcheck");
    }

    #[test]
    fn test_plan_toml_file_verify() {
        let files = vec![PathBuf::from("Cargo.toml")];
        let steps = VerificationPlanner::plan(&files);
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].target, "verify");
    }

    #[test]
    fn test_plan_md_file_non_critical() {
        let files = vec![PathBuf::from("README.md")];
        let steps = VerificationPlanner::plan(&files);
        assert_eq!(steps.len(), 1);
        assert!(!steps[0].critical);
    }

    #[test]
    fn test_plan_deduplicates() {
        let files = vec![PathBuf::from("src/main.rs"), PathBuf::from("src/lib.rs")];
        let steps = VerificationPlanner::plan(&files);
        let cargo_check = steps
            .iter()
            .filter(|s| s.tool == "cargo" && s.target == "check")
            .count();
        assert_eq!(cargo_check, 1);
    }

    #[test]
    fn test_steps_for_unknown_extension() {
        let steps = VerificationPlanner::steps_for_file(Path::new("data.json"));
        assert!(steps.is_empty());
    }

    #[test]
    fn test_should_skip_non_critical() {
        let step = VerificationStep {
            tool: "markdown".into(),
            target: "README.md".into(),
            timeout_secs: 30,
            critical: false,
        };
        assert!(VerificationPlanner::should_skip(&step));
    }

    #[test]
    fn test_should_not_skip_critical() {
        let step = VerificationStep {
            tool: "cargo".into(),
            target: "check".into(),
            timeout_secs: 120,
            critical: true,
        };
        assert!(!VerificationPlanner::should_skip(&step));
    }
}
