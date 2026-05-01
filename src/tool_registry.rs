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

    const DECLARATION_ONLY_TOOLS: &[&str] = &["edit", "write", "glob", "ls", "fetch", "patch"];
    const EXECUTABLE_TOOLS: &[&str] = &["shell", "read", "respond", "search", "summary"];

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
    fn test_declaration_only_tools_registered_but_not_callable() {
        let r = get_registry();

        // Declaration-only tools are registered (present in registry)
        for name in DECLARATION_ONLY_TOOLS {
            let def = r.get(name);
            assert!(
                def.is_some(),
                "declaration-only tool '{}' should be registered",
                name
            );
            assert_eq!(
                def.unwrap().executor_state,
                elma_tools::registry::ToolExecutorState::DeclarationOnly,
                "tool '{}' should be DeclarationOnly",
                name
            );
        }

        // Declaration-only tools are NOT returned by default_tools()
        let default_tools = r.default_tools();
        let default_names: Vec<&str> = default_tools
            .iter()
            .map(|t| t.function.name.as_str())
            .collect();
        for name in DECLARATION_ONLY_TOOLS {
            assert!(
                !default_names.contains(name),
                "declaration-only tool '{}' must not be in default_tools()",
                name
            );
        }

        // Declaration-only tools are NOT returned by get_tools()
        let decl_names: Vec<String> = DECLARATION_ONLY_TOOLS
            .iter()
            .map(|s| s.to_string())
            .collect();
        let callable = r.get_tools(&decl_names);
        assert!(
            callable.is_empty(),
            "declaration-only tools should not be returned by get_tools(): got {:?}",
            callable
                .iter()
                .map(|t| t.function.name.clone())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_executable_tools_are_callable() {
        let r = get_registry();

        let exec_names: Vec<String> = EXECUTABLE_TOOLS.iter().map(|s| s.to_string()).collect();
        let callable = r.get_tools(&exec_names);
        let callable_names: Vec<&str> = callable.iter().map(|t| t.function.name.as_str()).collect();

        for name in EXECUTABLE_TOOLS {
            assert!(
                callable_names.contains(name),
                "executable tool '{}' should be returned by get_tools()",
                name
            );
        }
    }

    #[test]
    fn test_declaration_only_not_in_build_current_tools() {
        let tools = build_current_tools();
        let names: Vec<&str> = tools.iter().map(|t| t.function.name.as_str()).collect();
        for name in DECLARATION_ONLY_TOOLS {
            assert!(
                !names.contains(name),
                "declaration-only tool '{}' must not be in build_current_tools()",
                name
            );
        }
    }
}
