use crate::dirs::ElmaPaths;
use crate::models_api::{load_provider_profile, ProviderProfile};
use crate::paths::{elma_config_path, project_elma_config_path};
use crate::types::ConfigAction;
use std::path::PathBuf;

pub(crate) fn handle_config_command(action: &ConfigAction, config_root: &str) {
    match action {
        ConfigAction::Path => cmd_path(),
        ConfigAction::Show => cmd_show(config_root),
        ConfigAction::Set { key, value } => cmd_set(key, value, config_root),
        ConfigAction::EffectiveProfile { profile_name } => cmd_effective_profile(profile_name),
        ConfigAction::Doctor => cmd_doctor(),
    }
}

fn find_provider_profile(config_root: &str) -> Option<ProviderProfile> {
    use crate::paths::discover_saved_base_url;
    let cfg_root = PathBuf::from(config_root);
    let base_url = discover_saved_base_url(&cfg_root, None).ok().flatten()
        .unwrap_or_else(|| "http://localhost:8080".to_string());
    // Scan model config subdirectories for provider_profile.toml
    if let Ok(entries) = std::fs::read_dir(&cfg_root) {
        for entry in entries.flatten() {
            let dir = entry.path();
            if dir.is_dir() {
                let profile_path = dir.join("provider_profile.toml");
                if profile_path.exists() {
                    if let Some(profile) = load_provider_profile(&dir) {
                        if profile.endpoint_url == base_url {
                            return Some(profile);
                        }
                    }
                }
            }
        }
    }
    None
}

fn cmd_path() {
    match elma_config_path() {
        Ok(p) => println!("{}", p.display()),
        Err(e) => eprintln!("Error: {}", e),
    }
}

fn cmd_show(config_root: &str) {
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
        let exists = if legacy.exists() {
            "exists"
        } else {
            "not found"
        };
        println!("  {} ({})", legacy.display(), exists);
    }

    println!("\nRuntime config:");
    let config_root = PathBuf::from(config_root);
    let runtime_path = crate::llm_config::runtime_config_path(&config_root);
    let exists = if runtime_path.exists() {
        "exists"
    } else {
        "not found"
    };
    println!("  {} ({})", runtime_path.display(), exists);
    if runtime_path.exists() {
        match crate::llm_config::load_or_create_runtime_llm_config(&config_root) {
            Ok(cfg) => {
                println!("  auxiliary_enabled = {}", cfg.auxiliary_enabled);
                println!("  auxiliary_base_url = {}", cfg.auxiliary_base_url);
                println!("  auxiliary_model = {}", cfg.auxiliary_model);
                println!("  auxiliary_timeout_s = {}", cfg.auxiliary_timeout_s);
            }
            Err(e) => println!("  (parse error: {})", e),
        }
    }

    // Show discovered provider profile
    if let Some(profile) = find_provider_profile(config_root.as_os_str().to_str().unwrap_or("")) {
        println!("\nProvider profile (discovered):");
        println!("  endpoint_url = {}", profile.endpoint_url);
        println!("  model_id = {}", profile.discovered_model_id);
        println!(
            "  context_window = {}",
            profile
                .context_window
                .map(|n| n.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        );
        println!("  provider = {}", profile.provider_family);
        println!("  runtime_kind = {}", profile.runtime_kind);
        println!("  supports_thinking = {}", profile.supports_thinking);
        println!("  supports_json_mode = {}", profile.supports_json_mode);
        println!("  probe_source = {}", profile.probe_source);
        println!(
            "  probe_age = {}s{}",
            profile.age_seconds(),
            if profile.is_stale() { " (STALE)" } else { "" },
        );
    } else {
        println!("\nProvider profile: not found (run /provider or start Elma to discover)");
    }
}

fn cmd_set(key: &str, value: &str, config_root: &str) {
    if key.starts_with("runtime.") {
        cmd_set_runtime(key, value, config_root);
        return;
    }

    match elma_config_path() {
        Ok(path) => {
            let mut cfg = path
                .exists()
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
                    eprintln!(
                        "Supported keys: provider.base_url, provider.model, runtime.auxiliary.enabled, runtime.auxiliary.base_url, runtime.auxiliary.model, runtime.auxiliary.timeout_s"
                    );
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

fn cmd_set_runtime(key: &str, value: &str, config_root: &str) {
    let config_root = PathBuf::from(config_root);
    let runtime_path = crate::llm_config::runtime_config_path(&config_root);
    let mut cfg = match crate::llm_config::load_or_create_runtime_llm_config(&config_root) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Error loading runtime config: {}", e);
            return;
        }
    };

    match key {
        "runtime.auxiliary.enabled" | "runtime.auxiliary_enabled" => match parse_bool(value) {
            Some(enabled) => cfg.auxiliary_enabled = enabled,
            None => {
                eprintln!("Invalid boolean value for {}: {}", key, value);
                eprintln!("Use true or false.");
                return;
            }
        },
        "runtime.auxiliary.base_url" | "runtime.auxiliary_base_url" => {
            cfg.auxiliary_base_url = value.to_string()
        }
        "runtime.auxiliary.model" | "runtime.auxiliary_model" => {
            cfg.auxiliary_model = value.to_string()
        }
        "runtime.auxiliary.timeout_s" | "runtime.auxiliary_timeout_s" => {
            let Ok(timeout_s) = value.parse::<u64>() else {
                eprintln!("Invalid integer value for {}: {}", key, value);
                return;
            };
            cfg.auxiliary_timeout_s = timeout_s;
        }
        _ => {
            eprintln!("Unknown runtime config key: {}", key);
            eprintln!(
                "Supported runtime keys: runtime.auxiliary.enabled, runtime.auxiliary.base_url, runtime.auxiliary.model, runtime.auxiliary.timeout_s"
            );
            return;
        }
    }

    let s = match toml::to_string_pretty(&cfg) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error serializing runtime config: {}", e);
            return;
        }
    };
    match std::fs::write(&runtime_path, s.as_bytes()) {
        Ok(_) => println!("Set {} = {} in {}", key, value, runtime_path.display()),
        Err(e) => eprintln!("Error writing runtime config: {}", e),
    }
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
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

    // Check provider profile freshness
    let config_root_str = ElmaPaths::new()
        .map(|p| p.config_dir().to_string_lossy().to_string())
        .unwrap_or_default();
    if !config_root_str.is_empty() {
        if let Some(profile) = find_provider_profile(&config_root_str) {
            if profile.is_stale() {
                issues.push(format!(
                    "Provider profile for {} is stale ({}s old, last probed via {})",
                    profile.endpoint_url,
                    profile.age_seconds(),
                    profile.probe_source,
                ));
            }
        } else {
            issues.push("No provider profile found — run /provider or start Elma to discover endpoint capabilities".to_string());
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
