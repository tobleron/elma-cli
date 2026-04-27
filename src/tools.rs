//! @efficiency-role: infra-adapter
//! Tools Module - Combined tools.rs (cache + registry)
//! discovery.rs remains as a separate submodule

pub mod discovery;
pub mod tool_evidence;

pub use discovery::*;
pub use tool_evidence::*;

// ── Cache types and functions ──────────────────────────────────────────────

use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCache {
    pub version: u32,
    pub cached_at: u64,
    pub path_hash: String,
    pub tools: Vec<CachedTool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedTool {
    pub name: String,
    pub path: String,
    pub category: String,
    pub description: String,
}

impl ToolCache {
    pub fn new() -> Self {
        Self {
            version: 1,
            cached_at: current_timestamp(),
            path_hash: String::new(),
            tools: Vec::new(),
        }
    }

    pub fn load(cache_path: &Path) -> Option<Self> {
        if !cache_path.exists() {
            return None;
        }

        let content = fs::read_to_string(cache_path).ok()?;
        serde_json::from_str(&content).ok()
    }

    pub fn save(&self, cache_path: &Path) -> Result<(), anyhow::Error> {
        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(cache_path, content)?;
        Ok(())
    }

    pub fn is_valid(&self, current_path_hash: &str) -> bool {
        // Cache valid for 7 days
        let age_seconds = current_timestamp() - self.cached_at;
        let seven_days = 7 * 24 * 60 * 60;

        self.path_hash == current_path_hash && age_seconds < seven_days
    }

    pub fn get_tools(&self) -> Vec<&CachedTool> {
        self.tools.iter().collect()
    }

    pub fn add_tool(&mut self, tool: CachedTool) {
        // Don't add duplicates
        if !self.tools.iter().any(|t| t.name == tool.name) {
            self.tools.push(tool);
        }
    }

    pub fn remove_missing(&mut self) {
        self.tools.retain(|tool| Path::new(&tool.path).exists());
    }
}

pub fn compute_path_hash() -> String {
    let mut hasher = DefaultHasher::new();

    // Hash PATH environment variable
    if let Ok(path) = std::env::var("PATH") {
        path.hash(&mut hasher);
    }

    // Hash PATHEXT on Windows
    #[cfg(windows)]
    if let Ok(pathext) = std::env::var("PATHEXT") {
        pathext.hash(&mut hasher);
    }

    format!("{:x}", hasher.finish())
}

pub fn get_cache_path() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());

    PathBuf::from(home)
        .join(".elma-cli")
        .join("cache")
        .join("tool_registry.json")
}

pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn verify_tool_exists(path: &str) -> bool {
    Path::new(path).exists()
}

pub fn merge_caches(old_cache: &ToolCache, new_tools: Vec<CachedTool>) -> ToolCache {
    let mut merged = ToolCache {
        version: old_cache.version,
        cached_at: current_timestamp(),
        path_hash: old_cache.path_hash.clone(),
        tools: old_cache.tools.clone(),
    };

    for tool in new_tools {
        merged.add_tool(tool);
    }

    merged
}

// ── Registry types and functions ───────────────────────────────────────────

use crate::tools::discovery::{discover_available_tools, ToolCapability, ToolCategory};

pub struct ToolRegistry {
    discovered: Vec<ToolCapability>,
    builtin: Vec<BuiltinStep>,
}

#[derive(Debug, Clone)]
pub struct BuiltinStep {
    pub name: String,
    pub description: String,
    pub step_type: String,
}

impl ToolRegistry {
    pub fn new(workspace: &Path) -> Self {
        // Use blocking executor for tool discovery (called once at startup)
        let discovered = futures::executor::block_on(discover_available_tools(workspace));

        let builtin = vec![
            BuiltinStep {
                name: "shell".to_string(),
                description: "Execute shell commands".to_string(),
                step_type: "shell".to_string(),
            },
            BuiltinStep {
                name: "read".to_string(),
                description: "Read file contents".to_string(),
                step_type: "read".to_string(),
            },
            BuiltinStep {
                name: "search".to_string(),
                description: "Search with ripgrep".to_string(),
                step_type: "search".to_string(),
            },
            BuiltinStep {
                name: "edit".to_string(),
                description: "Edit files".to_string(),
                step_type: "edit".to_string(),
            },
            BuiltinStep {
                name: "select".to_string(),
                description: "Select items from list".to_string(),
                step_type: "select".to_string(),
            },
            BuiltinStep {
                name: "reply".to_string(),
                description: "Respond to user".to_string(),
                step_type: "reply".to_string(),
            },
        ];

        Self {
            discovered,
            builtin,
        }
    }

    pub fn available_tools(&self) -> Vec<&ToolCapability> {
        self.discovered.iter().collect()
    }

    pub fn builtin_steps(&self) -> Vec<&BuiltinStep> {
        self.builtin.iter().collect()
    }

    pub fn describe_tool(&self, name: &str) -> Option<String> {
        // Check discovered tools
        if let Some(tool) = self.discovered.iter().find(|t| t.name == name) {
            return Some(format!(
                "{}: {} (Template: {})",
                tool.name, tool.description, tool.command_template
            ));
        }

        // Check builtin steps
        if let Some(step) = self.builtin.iter().find(|s| s.name == name) {
            return Some(format!(
                "{}: {} (Type: {})",
                step.name, step.description, step.step_type
            ));
        }

        None
    }

    pub fn format_tools_for_prompt(&self) -> String {
        let mut output = String::new();

        output.push_str("## Available Tools\n\n");

        // Builtin steps
        output.push_str("### Built-in Steps\n");
        for step in &self.builtin {
            output.push_str(&format!("- `{}`: {}\n", step.name, step.description));
        }

        output.push('\n');

        // CLI tools
        let cli_tools: Vec<_> = self
            .discovered
            .iter()
            .filter(|t| matches!(t.category, ToolCategory::CliTool))
            .collect();

        if !cli_tools.is_empty() {
            output.push_str("### CLI Tools\n");
            for tool in cli_tools {
                output.push_str(&format!("- `{}`: {}\n", tool.name, tool.description));
            }
            output.push('\n');
        }

        // Project-specific tools
        let project_tools: Vec<_> = self
            .discovered
            .iter()
            .filter(|t| matches!(t.category, ToolCategory::ProjectSpecific))
            .collect();

        if !project_tools.is_empty() {
            output.push_str("### Project Tools\n");
            for tool in project_tools {
                output.push_str(&format!("- `{}`: {}\n", tool.name, tool.description));
            }
            output.push('\n');
        }

        // Custom scripts
        let custom_scripts: Vec<_> = self
            .discovered
            .iter()
            .filter(|t| matches!(t.category, ToolCategory::CustomScript))
            .collect();

        if !custom_scripts.is_empty() {
            output.push_str("### Custom Scripts\n");
            for script in custom_scripts {
                output.push_str(&format!("- `{}`: {}\n", script.name, script.description));
            }
        }

        output
    }
}
