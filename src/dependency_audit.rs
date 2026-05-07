//! @efficiency-role: domain-logic
//!
//! Cargo dependency feature hygiene and supply risk audit (Task 674).
//! Scans Cargo.toml for dependency configuration issues.

use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SupplyRisk {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone)]
pub(crate) struct AuditReport {
    pub(crate) total_deps: usize,
    pub(crate) deps_with_default_features: Vec<String>,
    pub(crate) deps_with_all_features: Vec<String>,
    pub(crate) warnings: Vec<String>,
    pub(crate) supply_risk: SupplyRisk,
}

pub(crate) struct DependencyAudit;

impl DependencyAudit {
    pub(crate) fn audit(cargo_toml: &Path) -> AuditReport {
        let content = match std::fs::read_to_string(cargo_toml) {
            Ok(c) => c,
            Err(e) => {
                return AuditReport {
                    total_deps: 0,
                    deps_with_default_features: Vec::new(),
                    deps_with_all_features: Vec::new(),
                    warnings: vec![format!("Cannot read Cargo.toml: {}", e)],
                    supply_risk: SupplyRisk::Low,
                };
            }
        };
        let table = parse_toml_lite(&content);
        let deps = extract_dependencies(&table);
        let total_deps = deps.len();
        let mut deps_with_default_features = Vec::new();
        let mut deps_with_all_features = Vec::new();
        let mut warnings = Vec::new();
        let mut max_risk = SupplyRisk::Low;

        for (name, props) in &deps {
            let uses_default = props.get("default-features").map_or(true, |v| v == "true");
            if uses_default {
                deps_with_default_features.push(name.clone());
            }
            if props.get("features").map_or(false, |v| {
                v.contains("all") || v == "\"*\"" || v == "[\"*\"]"
            }) {
                deps_with_all_features.push(name.clone());
            }
            let version_req = props.get("version").cloned().unwrap_or_default();
            let risk = classify_supply_risk(name, &version_req);
            if risk_scores(&risk) > risk_scores(&max_risk) {
                max_risk = risk;
            }
        }

        if !deps_with_default_features.is_empty() {
            warnings.push(format!(
                "{} dependencies use default features (may pull unnecessary deps)",
                deps_with_default_features.len()
            ));
        }
        if !deps_with_all_features.is_empty() {
            warnings.push(format!(
                "{} dependencies use all features (consider enabling only needed features)",
                deps_with_all_features.len()
            ));
        }

        AuditReport {
            total_deps,
            deps_with_default_features,
            deps_with_all_features,
            warnings,
            supply_risk: max_risk,
        }
    }

    pub(crate) fn check_feature_hygiene(doc: &HashMap<String, String>) -> Vec<String> {
        let mut issues = Vec::new();
        for (key, val) in doc {
            if key == "default-features" && val == "true" {
                issues.push(format!("{} has default-features enabled", key));
            }
        }
        issues
    }
}

pub(crate) fn classify_supply_risk(dep_name: &str, version_req: &str) -> SupplyRisk {
    let version_req = version_req.trim_matches('"');
    if version_req.starts_with("git=") || version_req.starts_with("{ git") {
        if version_req.contains("rev")
            || version_req.contains("tag")
            || version_req.contains("branch")
        {
            return SupplyRisk::High;
        }
        return SupplyRisk::Critical;
    }
    if version_req.starts_with("*") {
        return SupplyRisk::Medium;
    }
    SupplyRisk::Low
}

fn risk_scores(risk: &SupplyRisk) -> u8 {
    match risk {
        SupplyRisk::Low => 0,
        SupplyRisk::Medium => 1,
        SupplyRisk::High => 2,
        SupplyRisk::Critical => 3,
    }
}

fn parse_toml_lite(content: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();
    let mut current_section = String::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            current_section = trimmed.trim_matches('[').trim_matches(']').to_string();
            continue;
        }
        if let Some(pos) = trimmed.find('=') {
            let key = trimmed[..pos].trim().to_string();
            let value = trimmed[pos + 1..].trim().to_string();
            let full_key = if current_section.is_empty() {
                key
            } else {
                format!("{}.{}", current_section, key)
            };
            result.insert(full_key, value);
        }
    }
    result
}

fn extract_dependencies(
    table: &HashMap<String, String>,
) -> HashMap<String, HashMap<String, String>> {
    let mut deps: HashMap<String, HashMap<String, String>> = HashMap::new();
    for (key, value) in table {
        let parts: Vec<&str> = key.splitn(2, '.').collect();
        if parts.len() < 2 {
            continue;
        }
        let section = parts[0];
        let prop = parts[1];
        if section == "dependencies"
            || section == "dev-dependencies"
            || section == "build-dependencies"
        {
            let dep_parts: Vec<&str> = prop.splitn(2, '.').collect();
            let dep_name = dep_parts[0].to_string();
            let entry = deps.entry(dep_name).or_default();
            if dep_parts.len() > 1 {
                entry.insert(dep_parts[1].to_string(), value.clone());
            } else {
                entry.insert("version".to_string(), value.clone());
            }
        }
    }
    deps
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn mock_cargo_toml() -> &'static str {
        r#"
[package]
name = "test"
version = "0.1.0"

[dependencies]
serde = "1.0"
reqwest = { version = "0.12", features = ["json"] }
tokio = { version = "1.37", default-features = false }

[dev-dependencies]
insta = "1.47"

[build-dependencies]
cc = "1.0"
"#
    }

    #[test]
    fn test_audit_counts_deps() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("Cargo.toml");
        fs::write(&path, mock_cargo_toml()).unwrap();
        let report = DependencyAudit::audit(&path);
        assert_eq!(report.total_deps, 5);
    }

    #[test]
    fn test_audit_default_features_warning() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("Cargo.toml");
        fs::write(&path, mock_cargo_toml()).unwrap();
        let report = DependencyAudit::audit(&path);
        assert!(!report.warnings.is_empty());
        assert!(report
            .deps_with_default_features
            .contains(&"serde".to_string()));
    }

    #[test]
    fn test_classify_supply_risk_low() {
        assert_eq!(classify_supply_risk("serde", "1.0"), SupplyRisk::Low);
    }

    #[test]
    fn test_classify_supply_risk_critical() {
        assert_eq!(
            classify_supply_risk("foo", "{ git = \"https://x/y\" }"),
            SupplyRisk::Critical
        );
    }

    #[test]
    fn test_classify_supply_risk_high_pinned_git() {
        assert_eq!(
            classify_supply_risk("foo", "{ git = \"https://x/y\", rev = \"abc123\" }"),
            SupplyRisk::High
        );
    }

    #[test]
    fn test_classify_supply_risk_medium_wildcard() {
        assert_eq!(classify_supply_risk("bar", "*"), SupplyRisk::Medium);
    }

    #[test]
    fn test_parse_toml_lite_sections() {
        let table = parse_toml_lite(mock_cargo_toml());
        assert!(table.contains_key("dependencies.serde"));
        assert!(table.contains_key("dev-dependencies.insta"));
        assert!(table.contains_key("build-dependencies.cc"));
    }

    #[test]
    fn test_check_feature_hygiene_finds_issues() {
        let mut doc = HashMap::new();
        doc.insert("default-features".to_string(), "true".to_string());
        let issues = DependencyAudit::check_feature_hygiene(&doc);
        assert_eq!(issues.len(), 1);
    }

    #[test]
    fn test_audit_missing_file_returns_defaults() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nonexistent.toml");
        let report = DependencyAudit::audit(&path);
        assert_eq!(report.total_deps, 0);
        assert!(!report.warnings.is_empty());
    }
}
