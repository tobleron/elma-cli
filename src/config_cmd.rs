use crate::dirs::ElmaPaths;
use crate::paths::{elma_config_path, project_elma_config_path};
use crate::types::ConfigAction;
use std::path::PathBuf;

pub(crate) fn handle_config_command(action: &ConfigAction) {
    match action {
        ConfigAction::Path => cmd_path(),
        ConfigAction::Show => cmd_show(),
        ConfigAction::Set { key, value } => cmd_set(key, value),
        ConfigAction::EffectiveProfile { profile_name } => cmd_effective_profile(profile_name),
        ConfigAction::Doctor => cmd_doctor(),
    }
}

fn cmd_path() {
    match elma_config_path() {
        Ok(p) => println!("{}", p.display()),
        Err(e) => eprintln!("Error: {}", e),
    }
}

fn cmd_show() {
    // Show OS-native config path
    println!("Global config path:");
    match elma_config_path() {
        Ok(p) => {
            let exists = if p.exists() { "exists" } else { "not found" };
            println!("  {} ({})", p.display(), exists);
            if p.exists() {
                match crate::paths::load_elma_config(&p) {
                    Ok(cfg) => {
                        println!("\n[provider]");
                        println!("  base_url = {}", cfg.base_url);
                        println!("  model = {}", cfg.model);
                    }
                    Err(e) => println!("  (parse error: {})", e),
                }
            }
        }
        Err(e) => println!("  error: {}", e),
    }

    // Show project-local config path
    println!("\nProject-local config path:");
    match project_elma_config_path() {
        Ok(p) => {
            let exists = if p.exists() { "exists" } else { "not found" };
            println!("  {} ({})", p.display(), exists);
        }
        Err(e) => println!("  error: {}", e),
    }

    // Show global.toml legacy path
    println!("\nLegacy global.toml:");
    if let Some(paths) = ElmaPaths::new() {
        let legacy = paths.config_dir().join("global.toml");
        let exists = if legacy.exists() { "exists" } else { "not found" };
        println!("  {} ({})", legacy.display(), exists);
    }
}

fn cmd_set(key: &str, value: &str) {
    match elma_config_path() {
        Ok(path) => {
            let mut cfg = path.exists()
                .then(|| crate::paths::load_elma_config(&path).ok())
                .flatten()
                .unwrap_or(crate::types::ElmaProjectConfig {
                    base_url: String::new(),
                    model: String::new(),
                });

            match key {
                "provider.base_url" => cfg.base_url = value.to_string(),
                "provider.model" => cfg.model = value.to_string(),
                _ => {
                    eprintln!("Unknown config key: {}", key);
                    eprintln!("Supported keys: provider.base_url, provider.model");
                    return;
                }
            }

            let s = toml::to_string_pretty(&cfg).unwrap_or_default();
            match std::fs::write(&path, s.as_bytes()) {
                Ok(_) => println!("Set {} = {} in {}", key, value, path.display()),
                Err(e) => eprintln!("Error writing config: {}", e),
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}

fn cmd_effective_profile(profile_name: &str) {
    // Read OS-native+project config to get base_url/model
    let base_url = crate::paths::discover_saved_base_url(
        &ElmaPaths::new()
            .map(|p| p.config_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("config")),
        None,
    )
    .ok()
    .flatten()
    .unwrap_or_else(|| "http://localhost:8080".to_string());

    // Look up the default profile
    let profile = default_profile(profile_name, &base_url, "");
    match profile {
        Some(p) => {
            println!("=== Effective profile: {} ===", profile_name);
            println!("version: {}", p.version);
            println!("name: {}", p.name);
            println!("base_url: {}", p.base_url);
            println!("model: {}", p.model);
            println!("temperature: {}", p.temperature);
            println!("top_p: {}", p.top_p);
            println!("repeat_penalty: {}", p.repeat_penalty);
            println!("reasoning_format: {}", p.reasoning_format);
            println!("max_tokens: {}", p.max_tokens);
            println!("timeout_s: {}", p.timeout_s);
        }
        None => {
            eprintln!("Unknown profile: {}", profile_name);
            eprintln!("Use one of: orchestrator, gate, router, summarizer, ...");
        }
    }
}

fn cmd_doctor() {
    let mut issues: Vec<String> = Vec::new();

    // Check OS-native config
    match elma_config_path() {
        Ok(p) => {
            if p.exists() {
                match crate::paths::load_elma_config(&p) {
                    Ok(cfg) => {
                        if cfg.base_url.is_empty() {
                            issues.push(format!("{}: base_url is empty", p.display()));
                        }
                    }
                    Err(e) => {
                        issues.push(format!("{}: parse error: {}", p.display(), e));
                    }
                }
            }
        }
        Err(e) => issues.push(format!("config path error: {}", e)),
    }

    // Check project-local override
    match project_elma_config_path() {
        Ok(p) => {
            if p.exists() {
                issues.push(format!("{}: project-local override present", p.display()));
            }
        }
        Err(_) => {}
    }

    // Check for legacy global.toml
    if let Some(paths) = ElmaPaths::new() {
        let legacy = paths.config_dir().join("global.toml");
        if legacy.exists() {
            issues.push(format!(
                "{}: legacy global.toml present (migrate to elma.toml)",
                legacy.display()
            ));
        }
    }

    if issues.is_empty() {
        println!("Config is healthy. No issues found.");
    } else {
        println!("Config issues:");
        for issue in &issues {
            println!("  - {}", issue);
        }
    }
}

/// Look up a profile by name from all built-in default registries.
/// Uses a macro to avoid fn pointer type coercion issues.
macro_rules! match_profile {
    ($name:expr, $base_url:expr, $model:expr; $($n:ident => $f:expr),+ $(,)?) => {
        $(
            if $name == stringify!($n) {
                let mut p = $f($base_url, $model);
                p.base_url = $base_url.to_string();
                return Some(p);
            }
        )+
    };
}

fn default_profile(name: &str, base_url: &str, model: &str) -> Option<crate::types::Profile> {
    match_profile!(name, base_url, model;
        _elma => crate::defaults_core::default_elma_config,
        intention => crate::defaults_core::default_intention_config,
        gate => crate::defaults_core::default_gate_config,
        gate_why => crate::defaults_core::default_gate_why_config,
        tooler => crate::defaults_core::default_tooler_config,
        orchestrator => crate::defaults_core::default_orchestrator_config,
        critic => crate::defaults_core::default_critic_config,
        program_repair => crate::defaults_core::default_program_repair_config,
        refinement => crate::defaults_core::default_refinement_config,
        reflection => crate::defaults_core::default_reflection_config,
        logical_reviewer => crate::defaults_core::default_logical_reviewer_config,
        logical_program_repair => crate::defaults_core::default_logical_program_repair_config,
        efficiency_reviewer => crate::defaults_core::default_efficiency_reviewer_config,
        efficiency_program_repair => crate::defaults_core::default_efficiency_program_repair_config,
        risk_reviewer => crate::defaults_core::default_risk_reviewer_config,
        meta_review => crate::defaults_core::default_meta_review_config,
    );

    match_profile!(name, base_url, model;
        router => crate::defaults_router::default_router_config,
        mode_router => crate::defaults_router::default_mode_router_config,
        speech_act => crate::defaults_router::default_speech_act_config,
        action_type => crate::defaults_router::default_action_type_config,
        planner_master => crate::defaults_router::default_planner_master_config,
        planner => crate::defaults_router::default_planner_config,
        decider => crate::defaults_router::default_decider_config,
        selector => crate::defaults_router::default_selector_config,
        summarizer => crate::defaults_router::default_summarizer_config,
        formatter => crate::defaults_router::default_formatter_config,
        json_outputter => crate::defaults_router::default_json_outputter_config,
        final_answer_extractor => crate::defaults_router::default_final_answer_extractor_config,
        calibration_judge => crate::defaults_router::default_calibration_judge_config,
        complexity_assessor => crate::defaults_router::default_complexity_assessor_config,
        evidence_need_assessor => crate::defaults_router::default_evidence_need_assessor_config,
        action_need_assessor => crate::defaults_router::default_action_need_assessor_config,
        pattern_suggester => crate::defaults_router::default_pattern_suggester_config,
        formula_selector => crate::defaults_router::default_formula_selector_config,
        formula_memory_matcher => crate::defaults_router::default_formula_memory_matcher_config,
        workflow_planner => crate::defaults_router::default_workflow_planner_config,
        workflow_complexity_planner => crate::defaults_router::default_workflow_complexity_planner_config,
        workflow_reason_planner => crate::defaults_router::default_workflow_reason_planner_config,
    );

    None
}
