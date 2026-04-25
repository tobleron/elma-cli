//! @efficiency-role: service-orchestrator
//!
//! Dynamic Tool Registry with Searchable Capabilities (Task 264)
//!
//! Implements Claude-Code-style dynamic tool discovery where:
//! - Tools have searchable capability hints (3-10 word phrases)
//! - Model uses ToolSearchTool to find/load tools dynamically
//! - Reduces prompt token usage by not including all tool schemas

use crate::types_api::{ToolDefinition, ToolFunction};
use std::collections::{HashMap, HashSet};
use std::sync::{OnceLock, RwLock};

/// Extended tool definition with searchable capability hints
#[derive(Debug, Clone)]
pub struct ToolDefinitionExt {
    pub tool_type: String,
    pub function: ToolFunction,
    /// Searchable capability hints (3-10 word phrases describing what this tool does)
    pub search_hints: Vec<String>,
    /// Whether this tool should be loaded by default (false for deferred tools)
    pub deferred: bool,
}

impl ToolDefinitionExt {
    pub fn new(name: &str, description: &str, parameters: serde_json::Value, hints: Vec<&str>) -> Self {
        Self {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: name.to_string(),
                description: description.to_string(),
                parameters: Some(parameters),
            },
            search_hints: hints.into_iter().map(|s| s.to_string()).collect(),
            deferred: true,
        }
    }

    pub fn not_deferred(mut self) -> Self {
        self.deferred = false;
        self
    }

    /// Convert to standard ToolDefinition for API calls
    pub fn to_tool_definition(&self) -> ToolDefinition {
        ToolDefinition {
            tool_type: self.tool_type.clone(),
            function: self.function.clone(),
        }
    }
}

/// Set of tools discovered via tool_search (dynamically loaded)
static DISCOVERED_TOOLS: OnceLock<RwLock<HashSet<String>>> = OnceLock::new();

fn discovered_tools() -> &'static RwLock<HashSet<String>> {
    DISCOVERED_TOOLS.get_or_init(|| RwLock::new(HashSet::new()))
}

/// Mark tools as discovered (available for use after tool_search)
/// Only deferred (non-default) tools are added to the discovered set.
pub fn mark_discovered(tool_names: &[String]) {
    let registry = get_registry();
    if let Ok(mut set) = discovered_tools().write() {
        for name in tool_names {
            // Only add if the tool exists and is deferred (not already in default set)
            if let Some(tool) = registry.get(name) {
                if tool.deferred {
                    set.insert(name.clone());
                }
            }
        }
    }
}

/// Get all discovered tool names
pub fn get_discovered() -> Vec<String> {
    discovered_tools()
        .read()
        .map(|set| set.iter().cloned().collect())
        .unwrap_or_default()
}

/// Dynamic Tool Registry with searchable capabilities
#[derive(Debug, Default)]
pub struct DynamicToolRegistry {
    tools: HashMap<String, ToolDefinitionExt>,
}

impl DynamicToolRegistry {
    pub fn new() -> Self {
        let mut registry = Self { tools: HashMap::new() };
        registry.register_default_tools();
        registry
    }

    fn register_default_tools(&mut self) {
        // ToolSearchTool - always available, not deferred
        self.tools.insert(
            "tool_search".to_string(),
            ToolDefinitionExt {
                tool_type: "function".to_string(),
                function: ToolFunction {
                    name: "tool_search".to_string(),
                    description: "Search for available tools by capability. Returns tool definitions that can be used in subsequent requests. Use this to discover tools needed for a task.".to_string(),
                    parameters: Some(serde_json::json!({
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "Search query describing the capability needed (e.g., 'read file', 'execute shell command', 'search text')"
                            }
                        },
                        "required": ["query"]
                    })),
                },
                search_hints: vec![
                    "search for tools".to_string(),
                    "discover available tools".to_string(),
                    "find tools by capability".to_string(),
                    "list available tools".to_string(),
                ],
                deferred: false, // Always available
            },
        );

        // Shell tool - deferred, loaded on demand
        self.tools.insert(
            "shell".to_string(),
            ToolDefinitionExt::new(
                "shell",
                "Execute a shell command and return its output.",
                serde_json::json!({
                    "type": "object",
                    "properties": {"command": {"type": "string"}},
                    "required": ["command"]
                }),
                vec![
                    "execute shell command",
                    "run command line",
                    "execute bash command",
                    "run terminal command",
                    "execute system command",
                ],
            ),
        );

        // Read tool - deferred
        self.tools.insert(
            "read".to_string(),
            ToolDefinitionExt::new(
                "read",
                "Read the contents of a file.",
                serde_json::json!({
                    "type": "object",
                    "properties": {"path": {"type": "string"}},
                    "required": ["path"]
                }),
                vec![
                    "read file contents",
                    "open file for reading",
                    "view file content",
                    "display file contents",
                ],
            ),
        );

        // Search tool - deferred
        self.tools.insert(
            "search".to_string(),
            ToolDefinitionExt::new(
                "search",
                "Search for text patterns in files using ripgrep.",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "pattern": {"type": "string"},
                        "path": {"type": "string"}
                    },
                    "required": ["pattern"]
                }),
                vec![
                    "search text in files",
                    "find pattern in code",
                    "grep search files",
                    "search file contents",
                    "find text pattern",
                ],
            ),
        );

        // Respond tool - always available (not deferred)
        self.tools.insert(
            "respond".to_string(),
            ToolDefinitionExt::new(
                "respond",
                "Provide a final answer to the user.",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "answer": {"type": "string"},
                        "content": {"type": "string"},
                        "text": {"type": "string"}
                    },
                    "anyOf": [
                        {"required": ["answer"]},
                        {"required": ["content"]},
                        {"required": ["text"]}
                    ]
                }),
                vec![
                    "provide final answer",
                    "respond to user",
                    "give answer to user",
                ],
            )
            .not_deferred(),
        );

        // Update todo list - deferred
        self.tools.insert(
            "update_todo_list".to_string(),
            ToolDefinitionExt::new(
                "update_todo_list",
                "Create and update a local task/todo list for multi-step work.",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "action": {"type":"string","enum":["add","update","in_progress","completed","blocked","remove","list"]},
                        "id": {"type":"integer"},
                        "text": {"type":"string"},
                        "reason": {"type":"string"}
                    },
                    "required": ["action"]
                }),
                vec![
                    "manage todo list",
                    "create task list",
                    "update task status",
                    "track tasks",
                ],
            ),
        );
    }

    /// Search tools by capability query
    pub fn search(&self, query: &str) -> Vec<&ToolDefinitionExt> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        for tool in self.tools.values() {
            // Search in tool name
            if tool.function.name.to_lowercase().contains(&query_lower) {
                results.push(tool);
                continue;
            }

            // Search in description
            if tool.function.description.to_lowercase().contains(&query_lower) {
                results.push(tool);
                continue;
            }

            // Search in capability hints
            for hint in &tool.search_hints {
                if hint.to_lowercase().contains(&query_lower) {
                    results.push(tool);
                    break;
                }
            }
        }

        results
    }

    /// Get tool by name
    pub fn get(&self, name: &str) -> Option<&ToolDefinitionExt> {
        self.tools.get(name)
    }

    /// Get all non-deferred tools (available by default)
    pub fn default_tools(&self) -> Vec<ToolDefinition> {
        self.tools
            .values()
            .filter(|t| !t.deferred)
            .map(|t| t.to_tool_definition())
            .collect()
    }

    /// Get tools by names
    pub fn get_tools(&self, names: &[String]) -> Vec<ToolDefinition> {
        names
            .iter()
            .filter_map(|name| self.tools.get(name))
            .map(|t| t.to_tool_definition())
            .collect()
    }

    /// Convert search results to tool definitions
    pub fn search_and_convert(&self, query: &str) -> Vec<ToolDefinition> {
        self.search(query)
            .into_iter()
            .map(|t| t.to_tool_definition())
            .collect()
    }

    /// Get all tool names (for search results)
    pub fn get_tool_names(&self, query: &str) -> Vec<String> {
        self.search(query)
            .into_iter()
            .map(|t| t.function.name.clone())
            .collect()
    }
}

/// Global registry instance
static REGISTRY: OnceLock<DynamicToolRegistry> = OnceLock::new();

pub fn get_registry() -> &'static DynamicToolRegistry {
    REGISTRY.get_or_init(DynamicToolRegistry::new)
}

/// Build the current tool definitions (default + discovered)
pub fn build_current_tools() -> Vec<ToolDefinition> {
    let registry = get_registry();
    let mut tools = registry.default_tools();
    
    // Add discovered tools
    let discovered = get_discovered();
    if !discovered.is_empty() {
        tools.extend(registry.get_tools(&discovered));
    }
    
    tools
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_contains_default_tools() {
        let registry = DynamicToolRegistry::new();
        assert!(registry.get("shell").is_some());
        assert!(registry.get("read").is_some());
        assert!(registry.get("search").is_some());
        assert!(registry.get("respond").is_some());
        assert!(registry.get("update_todo_list").is_some());
        assert!(registry.get("tool_search").is_some());
    }

    #[test]
    fn test_tool_search_returns_results() {
        let registry = DynamicToolRegistry::new();
        let results = registry.search("read file");
        assert!(!results.is_empty());
        assert!(results.iter().any(|t| t.function.name == "read"));
    }

    #[test]
    fn test_tool_search_by_description() {
        let registry = DynamicToolRegistry::new();
        let results = registry.search("execute shell command");
        assert!(!results.is_empty());
        assert!(results.iter().any(|t| t.function.name == "shell"));
    }

    #[test]
    fn test_tool_search_by_hints() {
        let registry = DynamicToolRegistry::new();
        let results = registry.search("find text pattern");
        assert!(!results.is_empty());
        assert!(results.iter().any(|t| t.function.name == "search"));
    }

    #[test]
    fn test_tool_search_no_results() {
        let registry = DynamicToolRegistry::new();
        let results = registry.search("nonexistent capability xyz");
        assert!(results.is_empty());
    }

    #[test]
    fn test_default_tools_excludes_deferred() {
        let registry = DynamicToolRegistry::new();
        let default_tools = registry.default_tools();
        let tool_names: Vec<String> = default_tools.iter().map(|t| t.function.name.clone()).collect();
        assert!(!tool_names.contains(&"shell".to_string()));
        assert!(!tool_names.contains(&"read".to_string()));
        assert!(!tool_names.contains(&"search".to_string()));
        assert!(!tool_names.contains(&"update_todo_list".to_string()));
        // tool_search and respond should be in default
        assert!(tool_names.contains(&"tool_search".to_string()));
        assert!(tool_names.contains(&"respond".to_string()));
    }

    #[test]
    fn test_get_tools_by_names() {
        let registry = DynamicToolRegistry::new();
        let tools = registry.get_tools(&["shell".to_string(), "read".to_string()].as_ref());
        assert_eq!(tools.len(), 2);
        let names: Vec<String> = tools.iter().map(|t| t.function.name.clone()).collect();
        assert!(names.contains(&"shell".to_string()));
        assert!(names.contains(&"read".to_string()));
    }

    #[test]
    fn test_search_and_convert() {
        let registry = DynamicToolRegistry::new();
        let tools = registry.search_and_convert("execute shell");
        assert!(!tools.is_empty());
        assert_eq!(tools[0].tool_type, "function");
    }

    #[test]
    fn test_get_tool_names() {
        let registry = DynamicToolRegistry::new();
        let names = registry.get_tool_names("read");
        assert_eq!(names.len(), 1);
        assert_eq!(names[0], "read");
    }

    #[test]
    fn test_build_current_tools_with_discovered() {
        // Clear discovered to start clean
        if let Ok(mut set) = discovered_tools().write() {
            set.clear();
        }

        let tools = build_current_tools();
        // Should only have default tools (tool_search, respond)
        let tool_names: Vec<String> = tools.iter().map(|t| t.function.name.clone()).collect();
        assert!(tool_names.contains(&"tool_search".to_string()));
        assert!(tool_names.contains(&"respond".to_string()));
        assert!(!tool_names.contains(&"shell".to_string()));
        assert!(!tool_names.contains(&"read".to_string()));
        assert!(!tool_names.contains(&"search".to_string()));
        assert!(!tool_names.contains(&"update_todo_list".to_string()));

        // Mark shell and read as discovered
        mark_discovered(&vec!["shell".to_string(), "read".to_string()]);
        let tools_with_discovered = build_current_tools();
        let names: Vec<String> = tools_with_discovered.iter().map(|t| t.function.name.clone()).collect();
        assert!(names.contains(&"shell".to_string()));
        assert!(names.contains(&"read".to_string()));
        // Defaults still present
        assert!(names.contains(&"tool_search".to_string()));
        assert!(names.contains(&"respond".to_string()));
    }

    #[test]
    fn test_search_hints_coverage() {
        let registry = DynamicToolRegistry::new();
        let shell = registry.get("shell").unwrap();
        assert!(!shell.search_hints.is_empty());
        assert!(shell.search_hints.iter().any(|h| h.contains("shell")));
    }
}
