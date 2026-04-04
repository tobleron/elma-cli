//! @efficiency-role: infra-adapter
//! Tool Registry - Manages discovered and builtin tools

use crate::tools::discovery::{discover_available_tools, ToolCapability, ToolCategory};
use std::path::Path;

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
