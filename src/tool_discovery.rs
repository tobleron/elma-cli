//! Autonomous Tool Discovery Module (Task 015)
//!
//! DESIGN RATIONALE:
//! Elma already knows common CLI tools from training (git, cargo, ls, etc.).
//! This module discovers WORKSPACE-SPECIFIC tools that the model cannot know:
//! - Custom scripts (./scripts/*.sh, *.py)
//! - Makefile targets
//! - npm scripts
//! - Justfile recipes
//! - Tool availability verification
//!
//! LAZY TRIGGERING:
//! Discovery only happens when explicitly requested or when needed,
//! not on every conversation turn.

use crate::*;
use std::collections::HashMap;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// A discovered tool capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCapability {
    /// Tool name (e.g., "deploy", "test_coverage")
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// How to invoke it (command template)
    pub invocation: String,
    /// Where this tool was discovered
    pub source: ToolSource,
    /// Whether the tool is currently available
    pub available: bool,
}

/// Where a tool was discovered
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolSource {
    /// Custom script file
    Script(PathBuf),
    /// Makefile target
    MakefileTarget,
    /// npm script from package.json
    NpmScript,
    /// Justfile recipe
    JustfileRecipe,
    /// System tool (verified installed)
    SystemTool,
}

/// Tool registry - lazily loaded
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolRegistry {
    /// Discovered tools indexed by name
    pub tools: HashMap<String, ToolCapability>,
    /// When the registry was last updated
    pub last_updated: Option<u64>,
    /// Whether discovery has been attempted
    pub discovery_attempted: bool,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            last_updated: None,
            discovery_attempted: false,
        }
    }
    
    /// Check if registry needs discovery
    pub fn needs_discovery(&self) -> bool {
        !self.discovery_attempted
    }
    
    /// Add a tool to the registry
    pub fn add_tool(&mut self, tool: ToolCapability) {
        self.tools.insert(tool.name.clone(), tool);
        self.last_updated = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        );
    }
    
    /// Get a tool by name
    pub fn get_tool(&self, name: &str) -> Option<&ToolCapability> {
        self.tools.get(name)
    }
    
    /// Format registry for display
    pub fn format_for_display(&self) -> String {
        if self.tools.is_empty() {
            return "No workspace-specific tools discovered.".to_string();
        }
        
        let mut output = String::from("=== Discovered Tools ===\n\n");
        
        // Group by source
        let mut scripts: Vec<&ToolCapability> = Vec::new();
        let mut makefile: Vec<&ToolCapability> = Vec::new();
        let mut npm: Vec<&ToolCapability> = Vec::new();
        let mut system: Vec<&ToolCapability> = Vec::new();
        
        for tool in self.tools.values() {
            match tool.source {
                ToolSource::Script(_) => scripts.push(tool),
                ToolSource::MakefileTarget => makefile.push(tool),
                ToolSource::NpmScript => npm.push(tool),
                ToolSource::JustfileRecipe => scripts.push(tool),
                ToolSource::SystemTool => system.push(tool),
            }
        }
        
        if !scripts.is_empty() {
            output.push_str("**Scripts:**\n");
            for tool in &scripts {
                output.push_str(&format!(
                    "- `{}`: {} ({})\n",
                    tool.name, tool.description, tool.invocation
                ));
            }
            output.push('\n');
        }
        
        if !makefile.is_empty() {
            output.push_str("**Makefile Targets:**\n");
            for tool in &makefile {
                output.push_str(&format!(
                    "- `make {}`: {}\n",
                    tool.name, tool.description
                ));
            }
            output.push('\n');
        }
        
        if !npm.is_empty() {
            output.push_str("**npm Scripts:**\n");
            for tool in &npm {
                output.push_str(&format!(
                    "- `npm run {}`: {}\n",
                    tool.name, tool.description
                ));
            }
            output.push('\n');
        }
        
        if !system.is_empty() {
            output.push_str("**Verified System Tools:**\n");
            for tool in &system {
                output.push_str(&format!("- `{}`: {}\n", tool.name, tool.description));
            }
            output.push('\n');
        }
        
        output
    }
}

/// Discover tools in the workspace (lazy trigger)
pub fn discover_workspace_tools(workspace_root: &Path) -> Result<ToolRegistry> {
    let mut registry = ToolRegistry::new();
    
    // Discover custom scripts
    discover_scripts(workspace_root, &mut registry)?;
    
    // Discover Makefile targets
    discover_makefile_targets(workspace_root, &mut registry)?;
    
    // Discover npm scripts
    discover_npm_scripts(workspace_root, &mut registry)?;
    
    // Verify common project tools
    verify_project_tools(workspace_root, &mut registry)?;
    
    registry.discovery_attempted = true;
    
    Ok(registry)
}

/// Discover custom scripts in common locations
fn discover_scripts(workspace_root: &Path, registry: &mut ToolRegistry) -> Result<()> {
    let script_dirs = ["scripts", "bin", "tools", ".scripts"];
    
    for dir_name in &script_dirs {
        let script_dir = workspace_root.join(dir_name);
        if !script_dir.exists() {
            continue;
        }
        
        if let Ok(entries) = std::fs::read_dir(&script_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                
                // Check if executable or has script extension
                let is_executable = path.metadata()
                    .map(|m| m.permissions().mode() & 0o111 != 0)
                    .unwrap_or(false);
                let has_script_ext = path.extension()
                    .map(|e| e == "sh" || e == "py" || e == "rb" || e == "js" || e == "ts")
                    .unwrap_or(false);
                
                if is_executable || has_script_ext {
                    let name = path.file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default();
                    
                    let rel_path = path.strip_prefix(workspace_root)
                        .unwrap_or(&path)
                        .to_string_lossy();
                    
                    registry.add_tool(ToolCapability {
                        name: name.clone(),
                        description: format!("Custom script at {}", rel_path),
                        invocation: format!("./{}", rel_path),
                        source: ToolSource::Script(path),
                        available: true,
                    });
                }
            }
        }
    }
    
    // Also check for root-level scripts
    let root_scripts = ["Makefile", "justfile", "Justfile"];
    for script in &root_scripts {
        if workspace_root.join(script).exists() {
            // Handled by other discovery functions
        }
    }
    
    Ok(())
}

/// Discover Makefile targets
fn discover_makefile_targets(workspace_root: &Path, registry: &mut ToolRegistry) -> Result<()> {
    let makefile_path = workspace_root.join("Makefile");
    if !makefile_path.exists() {
        return Ok(());
    }
    
    let content = std::fs::read_to_string(&makefile_path)?;
    
    // Simple regex to find targets (lines starting with word followed by colon)
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.starts_with('.') || line.starts_with('\t') {
            continue;
        }
        
        if let Some(colon_pos) = line.find(':') {
            let target = line[..colon_pos].trim();
            if !target.is_empty() && !target.contains('$') {
                // Skip common internal targets
                if !["all", "clean", "test", "build", "install", "uninstall", "help"].contains(&target) {
                    registry.add_tool(ToolCapability {
                        name: target.to_string(),
                        description: format!("Makefile target"),
                        invocation: format!("make {}", target),
                        source: ToolSource::MakefileTarget,
                        available: true,
                    });
                }
            }
        }
    }
    
    Ok(())
}

/// Discover npm scripts from package.json
fn discover_npm_scripts(workspace_root: &Path, registry: &mut ToolRegistry) -> Result<()> {
    let package_json_path = workspace_root.join("package.json");
    if !package_json_path.exists() {
        return Ok(());
    }
    
    let content = std::fs::read_to_string(&package_json_path)?;
    let package_json: serde_json::Value = serde_json::from_str(&content)?;
    
    if let Some(scripts) = package_json.get("scripts").and_then(|v| v.as_object()) {
        for (name, command) in scripts {
            registry.add_tool(ToolCapability {
                name: name.clone(),
                description: format!("npm script: {}", command.as_str().unwrap_or("")),
                invocation: format!("npm run {}", name),
                source: ToolSource::NpmScript,
                available: true,
            });
        }
    }
    
    Ok(())
}

/// Verify common project tools are available
fn verify_project_tools(workspace_root: &Path, registry: &mut ToolRegistry) -> Result<()> {
    // Check for Cargo.toml → cargo is relevant
    if workspace_root.join("Cargo.toml").exists() {
        let available = Command::new("cargo")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        
        registry.add_tool(ToolCapability {
            name: "cargo".to_string(),
            description: "Rust build tool and package manager".to_string(),
            invocation: "cargo <command>".to_string(),
            source: ToolSource::SystemTool,
            available,
        });
    }
    
    // Check for package.json → npm is relevant
    if workspace_root.join("package.json").exists() {
        let available = Command::new("npm")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        
        registry.add_tool(ToolCapability {
            name: "npm".to_string(),
            description: "Node.js package manager".to_string(),
            invocation: "npm <command>".to_string(),
            source: ToolSource::SystemTool,
            available,
        });
    }
    
    // Check for git repo
    if workspace_root.join(".git").exists() {
        let available = Command::new("git")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        
        registry.add_tool(ToolCapability {
            name: "git".to_string(),
            description: "Version control".to_string(),
            invocation: "git <command>".to_string(),
            source: ToolSource::SystemTool,
            available,
        });
    }
    
    Ok(())
}

/// Check if a command exists in PATH
pub fn command_exists(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_needs_discovery() {
        let registry = ToolRegistry::new();
        assert!(registry.needs_discovery());
        
        let mut registry = ToolRegistry::new();
        registry.discovery_attempted = true;
        assert!(!registry.needs_discovery());
    }

    #[test]
    fn test_add_tool() {
        let mut registry = ToolRegistry::new();
        registry.add_tool(ToolCapability {
            name: "test".to_string(),
            description: "test tool".to_string(),
            invocation: "test".to_string(),
            source: ToolSource::SystemTool,
            available: true,
        });
        
        assert!(registry.get_tool("test").is_some());
        assert!(registry.last_updated.is_some());
    }

    #[test]
    fn test_command_exists() {
        // Test with commands that exist on both macOS and Linux
        // Note: We use 'sh' as it's POSIX and always available
        assert!(command_exists("sh"));
        // Test a command that definitely doesn't exist
        assert!(!command_exists("nonexistent_command_xyz123"));
    }
}
