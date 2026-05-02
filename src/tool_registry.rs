//! @efficiency-role: service-orchestrator
//!
//! Thin wrapper around elma-tools crate.
//! Owns the global static registry and delegates all tool logic to elma_tools.

use elma_tools::{DynamicToolRegistry, ToolDefinition, ToolDefinitionExt, ToolFunction};
use std::sync::OnceLock;

/// Global registry instance
static REGISTRY: OnceLock<DynamicToolRegistry> = OnceLock::new();

pub fn get_registry() -> &'static DynamicToolRegistry {
    REGISTRY.get_or_init(DynamicToolRegistry::new)
}

// Re-export all public functions from elma-tools, delegating through the global registry.
// This preserves the existing call-site API (no argument changes needed).

pub fn mark_discovered(tool_names: &[String]) {
    // Only add deferred tools — filter through the registry
    let registry = get_registry();
    let deferred_names: std::collections::HashSet<String> = tool_names
        .iter()
        .filter(|name| registry.get(name).map(|t| t.deferred).unwrap_or(false))
        .cloned()
        .collect();
    elma_tools::mark_discovered_filtered(tool_names, &deferred_names);
}

pub fn get_discovered() -> Vec<String> {
    elma_tools::get_discovered()
}

pub fn build_current_tools() -> Vec<ToolDefinition> {
    elma_tools::build_current_tools(get_registry())
}

pub fn build_tools_for_context(context_hint: &str) -> Vec<ToolDefinition> {
    elma_tools::build_tools_for_context(get_registry(), context_hint)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrapper_registry_has_core_tools() {
        let r = get_registry();
        assert!(r.get("shell").is_some());
        assert!(r.get("read").is_some());
        assert!(r.get("respond").is_some());
    }

    #[test]
    fn test_wrapper_build_current_tools_includes_respond() {
        let tools = build_current_tools();
        let names: Vec<String> = tools.iter().map(|t| t.function.name.clone()).collect();
        assert!(names.contains(&"respond".to_string()));
    }

    #[test]
    fn test_tool_executor_parity() {
        let tools = crate::tool_registry::build_current_tools();
        let executor_handles = vec![
            "observe",
            "tool_search",
            "shell",
            "read",
            "glob",
            "patch",
            "search",
            "respond",
            "summary",
            "update_todo_list",
            "edit",
            "write",
        ];
        for name in executor_handles {
            assert!(
                tools.iter().any(|t| t.function.name == name),
                "tool {} is handled by executor but not in registry",
                name
            );
        }
    }
}
