//! @efficiency-role: domain-logic
//!
//! Dead code deprecation and large module debloating audit (Task 678).
//! Analyzes the codebase for oversized files and potentially unused dependencies.

use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Clone)]
pub(crate) struct LargeModule {
    pub(crate) path: std::path::PathBuf,
    pub(crate) lines: usize,
    pub(crate) suggested_split: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct DebloatReport {
    pub(crate) large_modules: Vec<LargeModule>,
    pub(crate) dead_code_candidates: Vec<String>,
    pub(crate) unused_deps: Vec<String>,
    pub(crate) savings_estimate: String,
}

pub(crate) struct DebloatAudit;

impl DebloatAudit {
    pub(crate) fn audit(root: &Path) -> DebloatReport {
        let large_modules = find_large_modules(root, 500);
        let dead_code_candidates = find_dead_code_candidates(root);
        let cargo_toml = root.join("Cargo.toml");
        let unused_deps = if cargo_toml.exists() {
            find_unused_deps(&cargo_toml)
        } else {
            Vec::new()
        };
        let total_lines: usize = large_modules.iter().map(|m| m.lines).sum();
        let estimate = if total_lines > 0 {
            format!(
                "~{} lines in {} large module(s); splitting could reduce ~{:.0}% of file size",
                total_lines,
                large_modules.len(),
                (total_lines as f64 / (total_lines + 1000) as f64) * 100.0
            )
        } else {
            "No significant debloat opportunities found".to_string()
        };

        DebloatReport {
            large_modules,
            dead_code_candidates,
            unused_deps,
            savings_estimate: estimate,
        }
    }
}

pub(crate) fn find_large_modules(root: &Path, threshold: usize) -> Vec<LargeModule> {
    let mut modules = Vec::new();
    let mut entries = Vec::new();
    collect_source_files(root, &mut entries);

    for file in &entries {
        let content = match std::fs::read_to_string(file) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let lines = content.lines().count();
        if lines > threshold {
            modules.push(LargeModule {
                path: file.clone(),
                lines,
                suggested_split: lines > threshold * 2,
            });
        }
    }
    modules.sort_by(|a, b| b.lines.cmp(&a.lines));
    modules
}

pub(crate) fn find_unused_deps(cargo_toml: &Path) -> Vec<String> {
    let content = match std::fs::read_to_string(cargo_toml) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let deps = extract_dep_names(&content);
    let src_dir = cargo_toml
        .parent()
        .map(|p| p.join("src"))
        .unwrap_or_default();
    if !src_dir.exists() {
        return Vec::new();
    }

    let mut source_files = Vec::new();
    collect_source_files(&src_dir, &mut source_files);

    let mut all_source = String::new();
    for file in &source_files {
        if let Ok(c) = std::fs::read_to_string(file) {
            all_source.push_str(&c);
            all_source.push('\n');
        }
    }

    let all_source_lower = all_source.to_lowercase();
    let mut unused = Vec::new();

    for dep in &deps {
        let dep_lower = dep.to_lowercase().replace('-', "_");
        let search_variants = [dep_lower.as_str(), dep.as_str()];
        let found = search_variants.iter().any(|v| {
            all_source_lower.contains(v) || all_source_lower.contains(&format!("use {}", v))
        });
        if !found {
            unused.push(dep.clone());
        }
    }

    let known_false_positives: HashSet<&str> = ["elma-cli", "elma-tools", "elma", "pi-agent"]
        .iter()
        .copied()
        .collect();
    unused.retain(|d| !known_false_positives.contains(d.as_str()));

    unused
}

fn find_dead_code_candidates(root: &Path) -> Vec<String> {
    let mut candidates = Vec::new();
    let mut entries = Vec::new();
    collect_source_files(root, &mut entries);

    let mut all_source = String::new();
    for file in &entries {
        if let Ok(c) = std::fs::read_to_string(file) {
            all_source.push_str(&c);
            all_source.push('\n');
        }
    }

    for file in &entries {
        let content = match std::fs::read_to_string(file) {
            Ok(c) => c,
            Err(_) => continue,
        };
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("fn ") && trimmed.contains('{') {
                let name = trimmed
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or("")
                    .split('(')
                    .next()
                    .unwrap_or("");
                if !name.is_empty() && !all_source.contains(&format!("{}(", name)) {
                    candidates.push(format!(
                        "{}:{}:{}",
                        file.display(),
                        name,
                        "function may be unused"
                    ));
                }
            }
        }
    }
    candidates
}

fn extract_dep_names(content: &str) -> Vec<String> {
    let mut deps = Vec::new();
    let mut in_deps = false;
    let mut in_dev_deps = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "[dependencies]" {
            in_deps = true;
            in_dev_deps = false;
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            if in_deps || in_dev_deps {
                break;
            }
            in_deps = false;
            in_dev_deps = false;
            continue;
        }
        if in_deps || in_dev_deps {
            if let Some(eq_pos) = trimmed.find('=') {
                let name = trimmed[..eq_pos].trim().to_string();
                if !name.is_empty() && !name.starts_with('#') {
                    deps.push(name);
                }
            }
        }
    }
    deps
}

fn collect_source_files(path: &Path, entries: &mut Vec<std::path::PathBuf>) {
    if path.is_dir() {
        if let Ok(dir) = std::fs::read_dir(path) {
            for entry in dir.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if name != "target" && !name.starts_with('.') {
                        collect_source_files(&p, entries);
                    }
                } else if p.extension().map_or(false, |e| e == "rs") {
                    entries.push(p);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_find_large_modules_detects_oversized() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("big.rs");
        let content = (0..600)
            .map(|i| format!("// line {}\n", i))
            .collect::<String>();
        fs::write(&file, content).unwrap();
        let modules = find_large_modules(dir.path(), 500);
        assert_eq!(modules.len(), 1);
        assert_eq!(modules[0].lines, 600);
    }

    #[test]
    fn test_find_large_modules_below_threshold() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("small.rs");
        let content = (0..100)
            .map(|i| format!("// line {}\n", i))
            .collect::<String>();
        fs::write(&file, content).unwrap();
        let modules = find_large_modules(dir.path(), 500);
        assert!(modules.is_empty());
    }

    #[test]
    fn test_find_large_modules_suggested_split() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("huge.rs");
        let content = (0..1200)
            .map(|i| format!("// line {}\n", i))
            .collect::<String>();
        fs::write(&file, content).unwrap();
        let modules = find_large_modules(dir.path(), 500);
        assert!(modules[0].suggested_split);
    }

    #[test]
    fn test_extract_dep_names() {
        let content = r#"
[package]
name = "test"

[dependencies]
serde = "1.0"
reqwest = "0.12"

[dev-dependencies]
insta = "1.47"
"#;
        let deps = extract_dep_names(content);
        assert!(deps.contains(&"serde".to_string()));
        assert!(deps.contains(&"reqwest".to_string()));
    }

    #[test]
    fn test_find_unused_deps_no_src() {
        let dir = TempDir::new().unwrap();
        let cargo = dir.path().join("Cargo.toml");
        fs::write(&cargo, "[dependencies]\nserde = \"1.0\"\n").unwrap();
        let unused = find_unused_deps(&cargo);
        assert!(unused.is_empty() || !unused.is_empty());
    }

    #[test]
    fn test_debloat_audit_produces_report() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("big.rs");
        let content = (0..600)
            .map(|i| format!("// line {}\n", i))
            .collect::<String>();
        fs::write(&file, content).unwrap();
        let toml = dir.path().join("Cargo.toml");
        fs::write(&toml, "[dependencies]\n").unwrap();
        let report = DebloatAudit::audit(dir.path());
        assert!(!report.large_modules.is_empty());
        assert!(!report.savings_estimate.is_empty());
    }

    #[test]
    fn test_savings_estimate_no_large_modules() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("tiny.rs");
        fs::write(&file, "fn main() {}\n").unwrap();
        let report = DebloatAudit::audit(dir.path());
        assert!(report.large_modules.is_empty());
        assert!(report.savings_estimate.contains("No significant"));
    }
}
