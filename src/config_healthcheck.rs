//! @efficiency-role: domain-logic
//!
//! Runtime Config Healthcheck (Task 069)
//!
//! Validates profiles, prompts, grammars, and global config at startup
//! so Elma fails early and clearly when runtime configuration is inconsistent.

use crate::app::LoadedProfiles;
use crate::*;

/// Result of a single healthcheck
#[derive(Debug, Clone)]
pub(crate) struct HealthCheckIssue {
    pub(crate) severity: HealthSeverity,
    pub(crate) component: String,
    pub(crate) message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum HealthSeverity {
    /// Config is broken and will cause runtime failure
    Error,
    /// Config is suboptimal but usable
    Warning,
}

/// Run startup config healthcheck on loaded profiles and assets.
/// Returns a list of issues (empty = healthy).
pub(crate) fn run_config_healthcheck(
    profiles: &LoadedProfiles,
    model_cfg_dir: &PathBuf,
    config_root: &str,
    global_cfg: &GlobalConfig,
) -> Vec<HealthCheckIssue> {
    let mut issues = Vec::new();

    // 1. Validate global config
    validate_global_config(global_cfg, &mut issues);

    // 2. Validate each loaded profile (exact fields from LoadedProfiles struct)
    validate_profile(&profiles.elma_cfg, "_elma.config", &mut issues);
    validate_profile(&profiles.intent_helper_cfg, "intent_helper", &mut issues);
    validate_profile(
        &profiles.expert_advisor_cfg,
        "expert_advisor",
        &mut issues,
    );
    validate_profile(
        &profiles.the_maestro_cfg,
        "the_maestro",
        &mut issues,
    );
    validate_profile(
        &profiles.status_message_cfg,
        "status_message_generator",
        &mut issues,
    );
    validate_profile(&profiles.planner_master_cfg, "planner_master", &mut issues);
    validate_profile(&profiles.planner_cfg, "planner", &mut issues);
    validate_profile(&profiles.decider_cfg, "decider", &mut issues);
    validate_profile(&profiles.selector_cfg, "selector", &mut issues);
    validate_profile(&profiles.summarizer_cfg, "summarizer", &mut issues);
    validate_profile(&profiles.turn_summary_cfg, "turn_summary", &mut issues);
    validate_profile(&profiles.formatter_cfg, "formatter", &mut issues);
    validate_profile(&profiles.json_outputter_cfg, "json_outputter", &mut issues);
    validate_profile(
        &profiles.final_answer_extractor_cfg,
        "final_answer_extractor",
        &mut issues,
    );
    validate_profile(
        &profiles.complexity_cfg,
        "complexity_assessment",
        &mut issues,
    );
    validate_profile(&profiles.evidence_need_cfg, "evidence_need", &mut issues);
    validate_profile(&profiles.action_need_cfg, "action_need", &mut issues);
    validate_profile(&profiles.formula_cfg, "formula", &mut issues);
    validate_profile(
        &profiles.workflow_planner_cfg,
        "workflow_planner",
        &mut issues,
    );
    validate_profile(
        &profiles.evidence_mode_cfg,
        "evidence_mode_selector",
        &mut issues,
    );
    validate_profile(&profiles.command_repair_cfg, "command_repair", &mut issues);
    validate_profile(
        &profiles.task_semantics_guard_cfg,
        "task_semantics_guard",
        &mut issues,
    );
    validate_profile(
        &profiles.execution_sufficiency_cfg,
        "execution_sufficiency",
        &mut issues,
    );
    validate_profile(
        &profiles.outcome_verifier_cfg,
        "outcome_verifier",
        &mut issues,
    );
    validate_profile(&profiles.memory_gate_cfg, "memory_gate", &mut issues);
    validate_profile(
        &profiles.command_preflight_cfg,
        "command_preflight",
        &mut issues,
    );
    validate_profile(&profiles.scope_builder_cfg, "scope_builder", &mut issues);
    validate_profile(
        &profiles.evidence_compactor_cfg,
        "evidence_compactor",
        &mut issues,
    );
    validate_profile(
        &profiles.artifact_classifier_cfg,
        "artifact_classifier",
        &mut issues,
    );
    validate_profile(
        &profiles.result_presenter_cfg,
        "result_presenter",
        &mut issues,
    );
    validate_profile(&profiles.claim_checker_cfg, "claim_checker", &mut issues);
    validate_profile(&profiles.orchestrator_cfg, "orchestrator", &mut issues);
    validate_profile(&profiles.critic_cfg, "critic", &mut issues);
    validate_profile(
        &profiles.logical_reviewer_cfg,
        "logical_reviewer",
        &mut issues,
    );
    validate_profile(
        &profiles.efficiency_reviewer_cfg,
        "efficiency_reviewer",
        &mut issues,
    );
    validate_profile(&profiles.risk_reviewer_cfg, "risk_reviewer", &mut issues);
    validate_profile(&profiles.refinement_cfg, "refinement", &mut issues);
    validate_profile(&profiles.reflection_cfg, "reflection", &mut issues);
    validate_profile(&profiles.meta_review_cfg, "meta_review", &mut issues);
    validate_profile(&profiles.router_cfg, "router_calibration", &mut issues);
    validate_profile(&profiles.mode_router_cfg, "mode_router", &mut issues);
    validate_profile(&profiles.speech_act_cfg, "speech_act", &mut issues);

    // 3. Validate grammar references
    validate_grammar_existence(config_root, &mut issues);

    // 4. Cross-profile consistency
    validate_cross_profile_consistency(profiles, global_cfg, &mut issues);

    issues
}

fn validate_global_config(cfg: &GlobalConfig, issues: &mut Vec<HealthCheckIssue>) {
    if cfg.base_url.is_empty() {
        issues.push(HealthCheckIssue {
            severity: HealthSeverity::Error,
            component: "global.toml".into(),
            message: "base_url is empty".into(),
        });
    } else if reqwest::Url::parse(&cfg.base_url).is_err() {
        issues.push(HealthCheckIssue {
            severity: HealthSeverity::Error,
            component: "global.toml".into(),
            message: format!("base_url '{}' is not a valid URL", cfg.base_url),
        });
    }
}

fn validate_profile(p: &Profile, name: &str, issues: &mut Vec<HealthCheckIssue>) {
    // Temperature must be in [0.0, 2.0]
    if p.temperature < 0.0 || p.temperature > 2.0 {
        issues.push(HealthCheckIssue {
            severity: HealthSeverity::Error,
            component: format!("{}.toml", name),
            message: format!("temperature {} is out of range [0.0, 2.0]", p.temperature),
        });
    }

    // top_p must be in (0.0, 1.0]
    if p.top_p <= 0.0 || p.top_p > 1.0 {
        issues.push(HealthCheckIssue {
            severity: HealthSeverity::Error,
            component: format!("{}.toml", name),
            message: format!("top_p {} is out of range (0.0, 1.0]", p.top_p),
        });
    }

    // max_tokens must be positive
    if p.max_tokens == 0 {
        issues.push(HealthCheckIssue {
            severity: HealthSeverity::Warning,
            component: format!("{}.toml", name),
            message: "max_tokens is 0 (will use server default)".into(),
        });
    }

    // repeat_penalty should be reasonable
    if p.repeat_penalty < 0.5 || p.repeat_penalty > 3.0 {
        issues.push(HealthCheckIssue {
            severity: HealthSeverity::Warning,
            component: format!("{}.toml", name),
            message: format!(
                "repeat_penalty {} is unusual (typical range: 0.5-3.0)",
                p.repeat_penalty
            ),
        });
    }

    // system_prompt must not be empty
    if p.system_prompt.trim().is_empty() {
        issues.push(HealthCheckIssue {
            severity: HealthSeverity::Error,
            component: format!("{}.toml", name),
            message: "system_prompt is empty".into(),
        });
    }

    // base_url being empty is acceptable (will be synced at startup)
    // but worth noting for debugging
}

fn validate_grammar_existence(config_root: &str, issues: &mut Vec<HealthCheckIssue>) {
    let mapping_path = std::path::Path::new(config_root).join("grammar_mapping.toml");
    if !mapping_path.exists() {
        issues.push(HealthCheckIssue {
            severity: HealthSeverity::Warning,
            component: "grammar_mapping".into(),
            message: "grammar_mapping.toml not found — grammar injection disabled".into(),
        });
        return;
    }

    // Parse and check each grammar file referenced in mapping
    if let Ok(mapping_content) = std::fs::read_to_string(&mapping_path) {
        for line in mapping_content.lines() {
            let line = line.trim();
            if line.starts_with('#') || line.is_empty() {
                continue;
            }
            if let Some(path_val) = line.split('=').nth(1).map(str::trim) {
                let path_val = path_val.trim_matches('"');
                let full_path = std::path::Path::new(config_root).join(path_val);
                if !full_path.exists() {
                    issues.push(HealthCheckIssue {
                        severity: HealthSeverity::Error,
                        component: "grammar".into(),
                        message: format!("Grammar file not found: {}", path_val),
                    });
                }
            }
        }
    }
}

fn validate_cross_profile_consistency(
    profiles: &LoadedProfiles,
    global_cfg: &GlobalConfig,
    issues: &mut Vec<HealthCheckIssue>,
) {
    // Check base_url agreement across key profiles
    let urls: Vec<(&str, &str)> = vec![
        ("_elma.config", &profiles.elma_cfg.base_url),
        ("expert_advisor", &profiles.expert_advisor_cfg.base_url),
        ("planner_master", &profiles.planner_master_cfg.base_url),
        ("planner", &profiles.planner_cfg.base_url),
    ];

    let first = urls.first().map(|(_, u)| *u).unwrap_or("");
    for (name, url) in &urls {
        if *url != first && !url.is_empty() {
            issues.push(HealthCheckIssue {
                severity: HealthSeverity::Warning,
                component: "cross_profile".into(),
                message: format!(
                    "Profiles disagree on base_url: '{}' vs '{}' (will be synced)",
                    first, url
                ),
            });
            break;
        }
    }

    // Global base_url should match profile base_url
    if !global_cfg.base_url.is_empty() && !profiles.elma_cfg.base_url.is_empty() {
        if global_cfg.base_url != profiles.elma_cfg.base_url {
            issues.push(HealthCheckIssue {
                severity: HealthSeverity::Warning,
                component: "cross_profile".into(),
                message: format!(
                    "global.toml base_url '{}' differs from _elma.config '{}'",
                    global_cfg.base_url, profiles.elma_cfg.base_url
                ),
            });
        }
    }
}

/// Format healthcheck results for startup display
pub(crate) fn format_healthcheck_report(issues: &[HealthCheckIssue]) -> String {
    if issues.is_empty() {
        return "  Config healthcheck: ✓ All profiles, grammars, and configs valid".into();
    }

    let mut report = String::new();
    let errors: Vec<_> = issues
        .iter()
        .filter(|i| i.severity == HealthSeverity::Error)
        .collect();
    let warnings: Vec<_> = issues
        .iter()
        .filter(|i| i.severity == HealthSeverity::Warning)
        .collect();

    if errors.is_empty() && warnings.is_empty() {
        report.push_str("  Config healthcheck: ✓ All valid\n");
    } else {
        report.push_str(&format!(
            "  Config healthcheck: {} error(s), {} warning(s)\n",
            errors.len(),
            warnings.len()
        ));
        for issue in &errors {
            report.push_str(&format!(
                "    [ERROR] {}: {}\n",
                issue.component, issue.message
            ));
        }
        for issue in &warnings {
            report.push_str(&format!(
                "    [WARN]  {}: {}\n",
                issue.component, issue.message
            ));
        }
    }

    report
}

/// Check if there are any errors (as opposed to just warnings)
pub(crate) fn has_config_errors(issues: &[HealthCheckIssue]) -> bool {
    issues.iter().any(|i| i.severity == HealthSeverity::Error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(unused_mut)]
    #[test]
    fn test_healthy_profile_no_issues() {
        let profile = Profile {
            version: 1,
            name: "test".into(),
            base_url: "http://localhost:8080".into(),
            model: "test".into(),
            temperature: 0.0,
            top_p: 1.0,
            repeat_penalty: 1.0,
            reasoning_format: "none".into(),
            max_tokens: 512,
            timeout_s: 120,
            system_prompt: "You are a test profile".into(),
        };
        let mut issues = Vec::new();
        validate_profile(&profile, "test", &mut issues);
        assert!(issues.is_empty(), "Expected no issues for healthy profile");
    }

    #[test]
    fn test_temperature_out_of_range() {
        let mut profile = Profile {
            version: 1,
            name: "test".into(),
            base_url: "http://localhost:8080".into(),
            model: "test".into(),
            temperature: 3.0,
            top_p: 1.0,
            repeat_penalty: 1.0,
            reasoning_format: "none".into(),
            max_tokens: 512,
            timeout_s: 120,
            system_prompt: "You are a test profile".into(),
        };
        let mut issues = Vec::new();
        validate_profile(&profile, "test", &mut issues);
        assert!(issues.iter().any(|i| i.message.contains("temperature")));
    }

    #[test]
    fn test_top_p_out_of_range() {
        let mut profile = Profile {
            version: 1,
            name: "test".into(),
            base_url: "http://localhost:8080".into(),
            model: "test".into(),
            temperature: 0.0,
            top_p: 0.0,
            repeat_penalty: 1.0,
            reasoning_format: "none".into(),
            max_tokens: 512,
            timeout_s: 120,
            system_prompt: "You are a test profile".into(),
        };
        let mut issues = Vec::new();
        validate_profile(&profile, "test", &mut issues);
        assert!(issues.iter().any(|i| i.message.contains("top_p")));
    }

    #[test]
    fn test_empty_system_prompt() {
        let mut profile = Profile {
            version: 1,
            name: "test".into(),
            base_url: "http://localhost:8080".into(),
            model: "test".into(),
            temperature: 0.0,
            top_p: 1.0,
            repeat_penalty: 1.0,
            reasoning_format: "none".into(),
            max_tokens: 512,
            timeout_s: 120,
            system_prompt: "".into(),
        };
        let mut issues = Vec::new();
        validate_profile(&profile, "test", &mut issues);
        assert!(issues.iter().any(|i| i.message.contains("system_prompt")));
    }

    #[test]
    fn test_invalid_global_base_url() {
        let global = GlobalConfig {
            version: 1,
            base_url: "not_a_url".into(),
        };
        let mut issues = Vec::new();
        validate_global_config(&global, &mut issues);
        assert!(issues.iter().any(|i| i.message.contains("not a valid URL")));
    }

    #[test]
    fn test_health_report_empty() {
        let report = format_healthcheck_report(&[]);
        assert!(report.contains("All profiles"));
        assert!(report.contains("valid"));
    }

    #[test]
    fn test_health_report_with_issues() {
        let issues = vec![
            HealthCheckIssue {
                severity: HealthSeverity::Error,
                component: "test.toml".into(),
                message: "test error".into(),
            },
            HealthCheckIssue {
                severity: HealthSeverity::Warning,
                component: "test2.toml".into(),
                message: "test warning".into(),
            },
        ];
        let report = format_healthcheck_report(&issues);
        assert!(report.contains("1 error"));
        assert!(report.contains("1 warning"));
        assert!(report.contains("test error"));
        assert!(report.contains("test warning"));
    }

    #[test]
    fn test_has_config_errors() {
        let no_errors = vec![HealthCheckIssue {
            severity: HealthSeverity::Warning,
            component: "test".into(),
            message: "warn".into(),
        }];
        assert!(!has_config_errors(&no_errors));

        let with_errors = vec![HealthCheckIssue {
            severity: HealthSeverity::Error,
            component: "test".into(),
            message: "err".into(),
        }];
        assert!(has_config_errors(&with_errors));
    }
}
