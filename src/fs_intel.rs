//! @efficiency-role: domain-logic
//!
//! Specialized Filesystem Intel (Task 072)
//!
//! Provides structured facts from config files to reduce token usage and improve accuracy.

use crate::*;

pub(crate) struct ProjectMetadata {
    pub(crate) language: Option<String>,
    pub(crate) package_name: Option<String>,
    pub(crate) version: Option<String>,
    pub(crate) entry_points: Vec<String>,
    pub(crate) dependencies: Vec<String>,
    pub(crate) scripts: Vec<(String, String)>,
}

impl ProjectMetadata {
    pub(crate) fn from_cargo_toml(content: &str) -> Self {
        let mut language = None;
        let mut package_name = None;
        let mut version = None;
        let mut dependencies = Vec::new();
        let mut scripts = Vec::new();

        if let Ok(toml) = content.parse::<toml::Value>() {
            if let Some(pkg) = toml.get("package") {
                if let Some(n) = pkg.get("name").and_then(|v| v.as_str()) {
                    package_name = Some(n.to_string());
                }
                if let Some(v) = pkg.get("version").and_then(|v| v.as_str()) {
                    version = Some(v.to_string());
                }
                if let Some(dt) = pkg.get("default-run").and_then(|v| v.as_str()) {
                    scripts.push(("default".to_string(), format!("cargo run -- {}", dt)));
                }
            }
            if let Some(deps) = toml.get("dependencies").and_then(|v| v.as_table()) {
                for (name, _) in deps {
                    dependencies.push(name.clone());
                }
            }
            if let Some(bin) = toml.get("bin").and_then(|v| v.as_array()) {
                for item in bin {
                    if let Some(path) = item.get("name").and_then(|v| v.as_str()) {
                        scripts.push((path.to_string(), format!("cargo run --bin {}", path)));
                    }
                }
            }
            if let Some(lib) = toml.get("lib").and_then(|v| v.as_table()) {
                if lib.contains_key("path") {
                    dependencies.push("lib".to_string());
                }
            }
            language = Some("rust".to_string());
        }

        ProjectMetadata {
            language,
            package_name,
            version,
            entry_points: scripts.iter().map(|(s, _)| s.clone()).collect(),
            dependencies,
            scripts,
        }
    }

    pub(crate) fn from_package_json(content: &str) -> Self {
        let mut language = None;
        let mut package_name = None;
        let mut version = None;
        let mut dependencies = Vec::new();
        let mut scripts = Vec::new();
        let mut entry_points = Vec::new();

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
            if let Some(name) = json.get("name").and_then(|v| v.as_str()) {
                package_name = Some(name.to_string());
            }
            if let Some(v) = json.get("version").and_then(|v| v.as_str()) {
                version = Some(v.to_string());
            }
            if let Some(deps) = json.get("dependencies").and_then(|v| v.as_object()) {
                for (name, _) in deps {
                    dependencies.push(name.clone());
                }
            }
            if let Some(dev_deps) = json.get("devDependencies").and_then(|v| v.as_object()) {
                for (name, _) in dev_deps {
                    dependencies.push(format!("dev:{}", name));
                }
            }
            if let Some(scrs) = json.get("scripts").and_then(|v| v.as_object()) {
                for (name, cmd) in scrs {
                    let cmd_str = cmd.as_str().unwrap_or("");
                    scripts.push((name.clone(), cmd_str.to_string()));
                    if name == "start" || name == "main" || name == "dev" {
                        entry_points.push(name.clone());
                    }
                }
            }
            if let Some(type_field) = json.get("type").and_then(|v| v.as_str()) {
                if type_field == "module" {
                    language = Some("javascript".to_string());
                }
            } else if let Some(main) = json.get("main").and_then(|v| v.as_str()) {
                entry_points.push(main.to_string());
                language = Some("javascript".to_string());
            }
            if language.is_none() {
                language = Some("javascript".to_string());
            }
        }

        ProjectMetadata {
            language,
            package_name,
            version,
            entry_points,
            dependencies,
            scripts,
        }
    }

    pub(crate) fn summary(&self) -> String {
        let mut parts = Vec::new();
        if let Some(ref lang) = self.language {
            parts.push(format!("language:{}", lang));
        }
        if let Some(ref name) = self.package_name {
            parts.push(format!("package:{}", name));
        }
        if let Some(ref ver) = self.version {
            parts.push(format!("v{}", ver));
        }
        if !self.dependencies.is_empty() {
            parts.push(format!("deps:{}", self.dependencies.len()));
        }
        if !self.scripts.is_empty() {
            let scripts_str = self
                .scripts
                .iter()
                .map(|(n, c)| format!("{}:{}", n, c))
                .collect::<Vec<_>>()
                .join(",");
            parts.push(format!("scripts:[{}]", scripts_str));
        }
        if !self.entry_points.is_empty() {
            parts.push(format!("entry:{}", self.entry_points.join(",")));
        }
        parts.join(" ")
    }
}

pub(crate) fn read_project_metadata(workspace: &Path) -> Option<ProjectMetadata> {
    let cargo_path = workspace.join("Cargo.toml");
    if cargo_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&cargo_path) {
            let meta = ProjectMetadata::from_cargo_toml(&content);
            if meta.package_name.is_some() {
                return Some(meta);
            }
        }
    }

    let package_path = workspace.join("package.json");
    if package_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&package_path) {
            let meta = ProjectMetadata::from_package_json(&content);
            if meta.package_name.is_some() {
                return Some(meta);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cargo_toml_parses_basic() {
        let content = r#"
[package]
name = "test-project"
version = "0.1.0"

[[bin]]
name = "main"
path = "src/main.rs"

[dependencies]
tokio = "1.0"
serde = "1.0"
"#;
        let meta = ProjectMetadata::from_cargo_toml(content);
        assert_eq!(meta.package_name, Some("test-project".to_string()));
        assert_eq!(meta.version, Some("0.1.0".to_string()));
        assert!(meta.dependencies.contains(&"tokio".to_string()));
    }

    #[test]
    fn package_json_parses_basic() {
        let content = r#"{
  "name": "my-app",
  "version": "1.2.3",
  "main": "dist/index.js",
  "scripts": {
    "start": "node dist/index.js",
    "build": "tsc"
  },
  "dependencies": {
    "express": "^4.0.0"
  },
  "devDependencies": {
    "typescript": "^5.0.0"
  }
}"#;
        let meta = ProjectMetadata::from_package_json(content);
        assert_eq!(meta.package_name, Some("my-app".to_string()));
        assert_eq!(meta.language, Some("javascript".to_string()));
        assert!(meta.dependencies.contains(&"express".to_string()));
        assert!(meta.dependencies.contains(&"dev:typescript".to_string()));
    }

    #[test]
    fn project_metadata_summary() {
        let meta = ProjectMetadata {
            language: Some("rust".to_string()),
            package_name: Some("demo".to_string()),
            version: Some("1.0.0".to_string()),
            entry_points: vec!["main".to_string()],
            dependencies: vec!["tokio".to_string()],
            scripts: vec![("run".to_string(), "cargo run".to_string())],
        };
        let summary = meta.summary();
        assert!(summary.contains("package:demo"));
        assert!(summary.contains("deps:1"));
    }
}
