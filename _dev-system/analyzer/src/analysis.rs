use crate::config::EfficiencyConfig;
use std::path::Path;

/// Calculate the dynamic LOC limit for a file based on its complexity metrics
pub fn calculate_dynamic_limit(
    drag: f64,
    p_mod: f64,
    cohesion_bonus: f64,
    dynamic_base: f64,
    config: &EfficiencyConfig,
    p_str: &str,
) -> usize {
    let mut limit = ((dynamic_base * p_mod * cohesion_bonus) / drag.powf(0.8))
        .max(config.settings.soft_floor_loc as f64) as usize;

    if let Some(exceptions) = &config.exceptions {
        for rule in exceptions {
            if p_str.contains(&rule.pattern) {
                if let Some(max) = rule.max_loc {
                    limit = max;
                }
                break;
            }
        }
    }
    limit.min(config.settings.hard_ceiling_loc)
}

/// Infer the taxonomy role of a file based on its path and content
pub fn infer_taxonomy(path: &Path, content: &str) -> String {
    use crate::drivers::{parse_header, EfficiencyOverride};

    let p = path.to_string_lossy().to_lowercase();
    let f = path
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");

    if ext != "json" && ext != "yaml" && ext != "toml" {
        match parse_header(content) {
            EfficiencyOverride::Ignore => return "ignored".to_string(),
            EfficiencyOverride::Role(name) => return name,
            _ => {}
        }
    }

    if f == "cargo.toml"
        || f == "package.json"
        || p.contains("/scripts/")
    {
        return "infra-config".to_string();
    }
    if f == "main.rs"
        || f == "lib.rs"
        || f == "mod.rs"
        || f == "app.rs"
        || f == "orchestration.rs"
        || f == "session.rs"
        || f == "index.js"
    {
        return "orchestrator".to_string();
    }
    if p.contains("/scenarios/") || f == "scenarios.rs" || f == "evaluation.rs" || f == "execution.rs" {
        return "scenario-spec".to_string();
    }
    if p.contains("/systems/") || p.contains("manager") || f == "workspace.rs" || f == "program.rs" {
        return "service-orchestrator".to_string();
    }
    if p.contains("/core/") && !p.contains("types") || f == "tune.rs" || f == "optimization.rs" || f == "intel.rs" {
        return "domain-logic".to_string();
    }
    if p.contains("/components/")
        || p.contains("view")
        || p.contains("/public/")
        || ext == "css"
        || f == "ui.rs"
    {
        return "ui-component".to_string();
    }
    if p.contains("reducer") || p.contains("state") {
        return "state-reducer".to_string();
    }
    if p.contains("types") || p.contains("models") || p.contains("schemas") || f == "defaults.rs" || f == "paths.rs" {
        return "data-model".to_string();
    }
    if p.contains("api") || p.contains("client") || p.contains("bindings") || p.contains("context") || f == "storage.rs" || f == "routing.rs"
    {
        return "infra-adapter".to_string();
    }
    if p.contains("utils") || p.contains("helpers") || f == "metrics.rs" || f == "tuning_support.rs" || f == "profile_sets.rs" {
        return "util-pure".to_string();
    }
    if ext == "toml" || ext == "json" || ext == "yaml" {
        return "infra-config".to_string();
    }
    "unknown".to_string()
}
