//! @efficiency-role: service-orchestrator
//!
//! Repo Explorer Skill — grounded repository structure analysis.
//! Maps structure, inspects representative files, and reports findings
//! with concrete file references.

use crate::*;
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct RepoOverview {
    pub root_files: Vec<String>,
    pub entry_points: Vec<String>,
    pub key_modules: Vec<ModuleHint>,
    pub manifest_files: Vec<String>,
    pub config_files: Vec<String>,
    pub risky_areas: Vec<String>,
    pub inspected_files: Vec<String>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ModuleHint {
    pub path: String,
    pub responsibility: String,
}

/// Build a grounded repo overview by inspecting representative files.
/// This is read-only and bounded — it does not deep-scan every file.
pub(crate) fn explore_repo(root: &Path) -> Result<RepoOverview> {
    let mut overview = RepoOverview::default();
    let mut inspected = HashSet::new();

    // 1. List root files
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.filter_map(|e| e.ok()) {
            let name = entry.file_name().to_string_lossy().to_string();
            overview.root_files.push(name);
        }
    }

    // 2. Discover manifest/build files
    let manifest_names = [
        "Cargo.toml",
        "package.json",
        "pyproject.toml",
        "setup.py",
        "go.mod",
        "pom.xml",
        "build.gradle",
        "CMakeLists.txt",
        "Makefile",
        "makefile",
    ];
    for name in &manifest_names {
        let path = root.join(name);
        if path.exists() {
            overview.manifest_files.push(name.to_string());
            if let Ok(content) = std::fs::read_to_string(&path) {
                inspected.insert(path.display().to_string());
                // Extract basic info from manifest
                if name == &"Cargo.toml" {
                    overview
                        .key_modules
                        .push(extract_cargo_modules(&content, root));
                }
            }
        }
    }

    // 3. Find likely entry points
    let entry_patterns = [
        "src/main.rs",
        "src/main.py",
        "src/main.go",
        "src/index.js",
        "src/index.ts",
        "main.rs",
        "main.py",
        "main.go",
        "index.js",
        "index.ts",
        "lib.rs",
        "lib.py",
        "lib.go",
    ];
    for pat in &entry_patterns {
        let path = root.join(pat);
        if path.exists() {
            overview.entry_points.push(pat.to_string());
            inspected.insert(path.display().to_string());
        }
    }

    // 4. Find config files
    let config_patterns = [
        ".gitignore",
        "README.md",
        "AGENTS.md",
        "config.toml",
        ".env",
        "docker-compose.yml",
        "Dockerfile",
    ];
    for pat in &config_patterns {
        let path = root.join(pat);
        if path.exists() {
            overview.config_files.push(pat.to_string());
            inspected.insert(path.display().to_string());
        }
    }

    // 5. Detect risky areas by scanning for large files or dense modules
    if let Ok(entries) = std::fs::read_dir(root.join("src")) {
        let mut large_files = Vec::new();
        for entry in entries.filter_map(|e| e.ok()) {
            if let Ok(meta) = entry.metadata() {
                if meta.len() > 100_000 {
                    large_files.push(format!(
                        "{} ({}KB)",
                        entry.file_name().to_string_lossy(),
                        meta.len() / 1024
                    ));
                }
            }
        }
        if !large_files.is_empty() {
            overview
                .risky_areas
                .push(format!("Large source files: {}", large_files.join(", ")));
        }
    }

    // 6. Inspect a bounded sample of representative source files
    let sample_dirs = ["src", "lib", "app", "core"];
    for dir_name in &sample_dirs {
        let dir = root.join(dir_name);
        if dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                let mut count = 0;
                for entry in entries.filter_map(|e| e.ok()) {
                    if count >= 3 {
                        break;
                    }
                    let path = entry.path();
                    if path
                        .extension()
                        .map(|e| e == "rs" || e == "py" || e == "go" || e == "js" || e == "ts")
                        .unwrap_or(false)
                    {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            let fname = path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string();
                            let lines = content.lines().count();
                            overview.key_modules.push(ModuleHint {
                                path: format!("{}/{}", dir_name, fname),
                                responsibility: format!("{} lines", lines),
                            });
                            inspected.insert(path.display().to_string());
                            count += 1;
                        }
                    }
                }
            }
        }
    }

    overview.inspected_files = inspected.into_iter().collect();
    overview.inspected_files.sort();
    overview.summary = format!(
        "Repository root has {} top-level entries. Found {} manifest(s), {} entry point(s), {} config file(s). Inspected {} representative files.",
        overview.root_files.len(),
        overview.manifest_files.len(),
        overview.entry_points.len(),
        overview.config_files.len(),
        overview.inspected_files.len()
    );

    Ok(overview)
}

fn extract_cargo_modules(content: &str, _root: &Path) -> ModuleHint {
    let has_workspace = content.contains("[workspace]");
    let bin_count = content.matches("[[bin]]").count();
    ModuleHint {
        path: "Cargo.toml".to_string(),
        responsibility: format!(
            "Rust project; {}workspace; {} bin target(s)",
            if has_workspace { "" } else { "non-" },
            bin_count.max(1)
        ),
    }
}

pub(crate) fn render_repo_overview(overview: &RepoOverview) -> String {
    let mut lines = Vec::new();
    lines.push("REPO OVERVIEW".to_string());
    lines.push(overview.summary.clone());
    if !overview.manifest_files.is_empty() {
        lines.push(format!("Manifests: {}", overview.manifest_files.join(", ")));
    }
    if !overview.entry_points.is_empty() {
        lines.push(format!(
            "Entry points: {}",
            overview.entry_points.join(", ")
        ));
    }
    if !overview.key_modules.is_empty() {
        lines.push("Key modules:".to_string());
        for m in &overview.key_modules {
            lines.push(format!("  {} — {}", m.path, m.responsibility));
        }
    }
    if !overview.risky_areas.is_empty() {
        lines.push("Risky areas:".to_string());
        for r in &overview.risky_areas {
            lines.push(format!("  {}", r));
        }
    }
    lines.push(format!(
        "Inspected files ({}):",
        overview.inspected_files.len()
    ));
    for f in &overview.inspected_files {
        lines.push(format!("  {}", f));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explore_current_repo_finds_manifests() {
        let root = Path::new(".");
        let overview = explore_repo(root).unwrap();
        assert!(
            overview.manifest_files.iter().any(|m| m == "Cargo.toml"),
            "should find Cargo.toml"
        );
        assert!(
            !overview.inspected_files.is_empty(),
            "should inspect some files"
        );
    }

    #[test]
    fn render_overview_includes_inspected_list() {
        let overview = RepoOverview {
            manifest_files: vec!["Cargo.toml".to_string()],
            inspected_files: vec!["Cargo.toml".to_string(), "src/main.rs".to_string()],
            summary: "Test repo".to_string(),
            ..Default::default()
        };
        let rendered = render_repo_overview(&overview);
        assert!(rendered.contains("Cargo.toml"));
        assert!(rendered.contains("Inspected files"));
    }
}
