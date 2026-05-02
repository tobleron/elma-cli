use crate::types::{ToolDefinition, ToolFunction};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock, RwLock};

/// How a tool is implemented — used for native-over-shell preference ranking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImplementationKind {
    /// Pure Rust, no external runtime dependencies
    RustNative,
    /// Rust wrapper around a system binary/library
    RustWrapper,
    /// Executes shell commands (bash, sh, etc.)
    Shell,
    /// Requires network access
    Network,
    /// External extension (MCP, plugin, etc.)
    External,
}

impl ImplementationKind {
    /// Priority for tool selection (higher = preferred).
    /// Rust-native and rust-wrapper are preferred over shell and network.
    pub fn selection_priority(&self) -> u8 {
        match self {
            Self::RustNative => 100,
            Self::RustWrapper => 90,
            Self::Shell => 40,
            Self::Network => 30,
            Self::External => 20,
        }
    }

    /// Whether this tool works offline.
    pub fn is_offline_capable(&self) -> bool {
        matches!(self, Self::RustNative | Self::RustWrapper | Self::Shell)
    }
}

/// Extended tool definition with searchable capability hints and prerequisite check.
#[derive(Clone)]
pub struct ToolDefinitionExt {
    pub tool_type: String,
    pub function: ToolFunction,
    /// Searchable capability hints (3-10 word phrases describing what this tool does)
    pub search_hints: Vec<String>,
    /// Whether this tool should be loaded by default (false for deferred tools)
    pub deferred: bool,
    /// How this tool is implemented (for native-over-shell ranking)
    pub implementation_kind: ImplementationKind,
    /// Whether this tool is workspace-scoped (operates within workspace boundaries)
    pub workspace_scoped: bool,
    /// Shell command families this tool replaces (e.g., "ls", "cat", "grep", "find")
    /// Used to prefer the native tool over shell for equivalent operations.
    pub shell_equivalents: Vec<String>,
    /// Optional prerequisite check. Returns true if runtime dependencies are available.
    /// None means "always available".
    #[allow(clippy::type_complexity)]
    pub check_fn: Option<Arc<dyn Fn() -> bool + Send + Sync>>,
    /// Policy metadata for this tool.
    pub policy: ToolPolicy,
}

impl std::fmt::Debug for ToolDefinitionExt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolDefinitionExt")
            .field("tool_type", &self.tool_type)
            .field("function", &self.function)
            .field("search_hints", &self.search_hints)
            .field("deferred", &self.deferred)
            .field("implementation_kind", &self.implementation_kind)
            .field("workspace_scoped", &self.workspace_scoped)
            .field("shell_equivalents", &self.shell_equivalents)
            .field("check_fn", &self.check_fn.as_ref().map(|_| "<closure>"))
            .field("policy", &self.policy)
            .finish()
    }
}

impl ToolDefinitionExt {
    pub fn new(
        name: &str,
        description: &str,
        parameters: serde_json::Value,
        hints: Vec<&str>,
    ) -> Self {
        Self {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: name.to_string(),
                description: description.to_string(),
                parameters: Some(parameters),
            },
            search_hints: hints.into_iter().map(|s| s.to_string()).collect(),
            deferred: true,
            implementation_kind: ImplementationKind::RustNative,
            workspace_scoped: true,
            shell_equivalents: Vec::new(),
            check_fn: None,
            policy: ToolPolicy::default(),
        }
    }

    pub fn not_deferred(mut self) -> Self {
        self.deferred = false;
        self
    }

    pub fn deferred(mut self) -> Self {
        self.deferred = true;
        self
    }

    /// Set the implementation kind (native, shell, network, etc.)
    pub fn with_implementation(mut self, kind: ImplementationKind) -> Self {
        self.implementation_kind = kind;
        self
    }

    /// Mark this tool as not workspace-scoped (e.g., fetch, network tools)
    pub fn not_workspace_scoped(mut self) -> Self {
        self.workspace_scoped = false;
        self
    }

    /// Set shell command families this tool replaces.
    pub fn with_shell_equivalents(mut self, equivalents: Vec<&str>) -> Self {
        self.shell_equivalents = equivalents.into_iter().map(|s| s.to_string()).collect();
        self
    }

    /// Set a prerequisite check function. Returns self for builder pattern.
    pub fn with_check_fn(mut self, f: impl Fn() -> bool + Send + Sync + 'static) -> Self {
        self.check_fn = Some(Arc::new(f));
        self
    }

    /// Set policy metadata.
    pub fn with_policy(mut self, policy: ToolPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Set risks for this tool.
    pub fn with_risks(mut self, risks: Vec<ToolRisk>) -> Self {
        self.policy.risks = risks;
        self
    }

    /// Set whether this tool requires permission.
    pub fn requires_permission(mut self, required: bool) -> Self {
        self.policy.requires_permission = required;
        self
    }

    /// Set whether this tool requires prior read.
    pub fn requires_prior_read(mut self, required: bool) -> Self {
        self.policy.requires_prior_read = required;
        self
    }

    /// Set whether this tool is concurrency-safe.
    pub fn concurrency_safe(mut self, safe: bool) -> Self {
        self.policy.concurrency_safe = safe;
        self
    }

    /// Set executor state.
    pub fn with_executor_state(mut self, state: ExecutorState) -> Self {
        self.policy.executor_state = state;
        self
    }

    /// Set whether this tool mutates workspace.
    pub fn mutates_workspace(mut self, mutates: bool) -> Self {
        self.policy.mutates_workspace = mutates;
        self
    }

    /// Set whether this tool creates artifacts.
    pub fn creates_artifacts(mut self, creates: bool) -> Self {
        self.policy.creates_artifacts = creates;
        self
    }

    /// Check if this tool's prerequisites are met. None means always available.
    pub fn is_available(&self) -> bool {
        match &self.check_fn {
            Some(check) => check(),
            None => true,
        }
    }

    /// Convert to standard ToolDefinition for API calls
    pub fn to_tool_definition(&self) -> ToolDefinition {
        ToolDefinition {
            tool_type: self.tool_type.clone(),
            function: self.function.clone(),
        }
    }
}

/// Builder for assembling tool registries from per-tool modules.
pub struct RegistryBuilder {
    tools: HashMap<String, ToolDefinitionExt>,
}

impl RegistryBuilder {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn insert(&mut self, tool: ToolDefinitionExt) {
        let name = tool.function.name.clone();
        self.tools.insert(name, tool);
    }

    pub fn build(self) -> DynamicToolRegistry {
        DynamicToolRegistry { tools: self.tools }
    }
}

impl Default for RegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Set of tools discovered via tool_search (dynamically loaded)
static DISCOVERED_TOOLS: OnceLock<RwLock<HashSet<String>>> = OnceLock::new();

fn discovered_tools() -> &'static RwLock<HashSet<String>> {
    DISCOVERED_TOOLS.get_or_init(|| RwLock::new(HashSet::new()))
}

/// Mark tools as discovered (available for use after tool_search).
/// Only deferred (non-default) tools are added to the discovered set.
pub fn mark_discovered(tool_names: &[String]) {
    // We can't access the global registry from the crate level without creating
    // a circular dependency on the main crate. The main crate holds the global
    // static and calls this function after checking against the registry.
    if let Ok(mut set) = discovered_tools().write() {
        for name in tool_names {
            set.insert(name.clone());
        }
    }
}

/// Mark tools as discovered with deferral check (called from main crate).
pub fn mark_discovered_filtered(tool_names: &[String], deferred_names: &HashSet<String>) {
    if let Ok(mut set) = discovered_tools().write() {
        for name in tool_names {
            if deferred_names.contains(name.as_str()) {
                set.insert(name.clone());
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
        let mut builder = RegistryBuilder::new();
        crate::tools::register_all(&mut builder);
        builder.build()
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
            if tool
                .function
                .description
                .to_lowercase()
                .contains(&query_lower)
            {
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

    /// Get all non-deferred tools (available by default), sorted by name for cache stability.
    /// Filters out tools whose check_fn returns false (missing prerequisites).
    pub fn default_tools(&self) -> Vec<ToolDefinition> {
        let mut tools: Vec<ToolDefinition> = self
            .tools
            .values()
            .filter(|t| !t.deferred && t.is_available())
            .map(|t| t.to_tool_definition())
            .collect();
        tools.sort_by(|a, b| a.function.name.cmp(&b.function.name));
        tools
    }

    /// Get all tools whose prerequisites are met, regardless of deferred status.
    pub fn available_tools(&self) -> Vec<&ToolDefinitionExt> {
        self.tools.values().filter(|t| t.is_available()).collect()
    }

    /// Get tools by names, sorted by name for cache stability.
    /// Filters out tools whose prerequisites are not met.
    pub fn get_tools(&self, names: &[String]) -> Vec<ToolDefinition> {
        let mut tools: Vec<ToolDefinition> = names
            .iter()
            .filter_map(|name| self.tools.get(name))
            .filter(|t| t.is_available())
            .map(|t| t.to_tool_definition())
            .collect();
        tools.sort_by(|a, b| a.function.name.cmp(&b.function.name));
        tools
    }

    /// Convert search results to tool definitions.
    /// Filters out tools whose prerequisites are not met.
    pub fn search_and_convert(&self, query: &str) -> Vec<ToolDefinition> {
        self.search(query)
            .into_iter()
            .filter(|t| t.is_available())
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

/// Build the current tool definitions (default + discovered), sorted by name.
pub fn build_current_tools(registry: &DynamicToolRegistry) -> Vec<ToolDefinition> {
    let mut tools = registry.default_tools();

    // Add discovered tools
    let discovered = get_discovered();
    if !discovered.is_empty() {
        tools.extend(registry.get_tools(&discovered));
    }

    // Stable ordering for prompt caching
    tools.sort_by(|a, b| a.function.name.cmp(&b.function.name));
    tools.dedup_by(|a, b| a.function.name == b.function.name);
    tools
}

/// Build tool definitions filtered by task context (route/classification).
pub fn build_tools_for_context(
    registry: &DynamicToolRegistry,
    context_hint: &str,
) -> Vec<ToolDefinition> {
    let context = context_hint.to_lowercase();

    let allowed_names: Vec<String> = match context.as_str() {
        "chat" => vec!["respond".to_string(), "summary".to_string()],
        "shell" => vec![
            "read".to_string(),
            "respond".to_string(),
            "summary".to_string(),
            "search".to_string(),
            "shell".to_string(),
            "tool_search".to_string(),
            "update_todo_list".to_string(),
        ],
        "plan" => vec![
            "read".to_string(),
            "respond".to_string(),
            "summary".to_string(),
            "search".to_string(),
            "tool_search".to_string(),
            "update_todo_list".to_string(),
        ],
        "decide" => vec![
            "read".to_string(),
            "respond".to_string(),
            "summary".to_string(),
            "search".to_string(),
            "update_todo_list".to_string(),
        ],
        _ => {
            return build_current_tools(registry);
        }
    };

    let mut tools = registry.get_tools(&allowed_names);
    if !tools.iter().any(|t| t.function.name == "respond") {
        if let Some(respond) = registry.get("respond") {
            tools.push(respond.to_tool_definition());
        }
    }
    tools.sort_by(|a, b| a.function.name.cmp(&b.function.name));
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
    fn test_default_tools_includes_core_tools() {
        let registry = DynamicToolRegistry::new();
        let default_tools = registry.default_tools();
        let tool_names: Vec<String> = default_tools
            .iter()
            .map(|t| t.function.name.clone())
            .collect();
        assert!(tool_names.contains(&"shell".to_string()));
        assert!(tool_names.contains(&"read".to_string()));
        assert!(tool_names.contains(&"search".to_string()));
        assert!(tool_names.contains(&"update_todo_list".to_string()));
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
        let names = registry.get_tool_names("read file contents");
        assert!(!names.is_empty());
        assert!(names.contains(&"read".to_string()));
    }

    #[test]
    fn test_build_current_tools_includes_all_core() {
        if let Ok(mut set) = discovered_tools().write() {
            set.clear();
        }

        let registry = DynamicToolRegistry::new();
        let tools = build_current_tools(&registry);
        let tool_names: Vec<String> = tools.iter().map(|t| t.function.name.clone()).collect();
        assert!(tool_names.contains(&"shell".to_string()));
        assert!(tool_names.contains(&"read".to_string()));
        assert!(tool_names.contains(&"search".to_string()));
        assert!(tool_names.contains(&"update_todo_list".to_string()));
        assert!(tool_names.contains(&"tool_search".to_string()));
        assert!(tool_names.contains(&"respond".to_string()));
    }

    #[test]
    fn test_search_hints_coverage() {
        let registry = DynamicToolRegistry::new();
        let shell = registry.get("shell").unwrap();
        assert!(!shell.search_hints.is_empty());
        assert!(shell.search_hints.iter().any(|h| h.contains("shell")));
    }

    #[test]
    fn test_build_tools_for_context_chat() {
        let registry = DynamicToolRegistry::new();
        let tools = build_tools_for_context(&registry, "chat");
        let names: Vec<String> = tools.iter().map(|t| t.function.name.clone()).collect();
        assert!(names.contains(&"respond".to_string()));
        assert!(!names.contains(&"shell".to_string()));
        assert!(!names.contains(&"read".to_string()));
        assert_eq!(tools.first().unwrap().function.name, "respond");
    }

    #[test]
    fn test_build_tools_for_context_shell() {
        let registry = DynamicToolRegistry::new();
        let tools = build_tools_for_context(&registry, "shell");
        let names: Vec<String> = tools.iter().map(|t| t.function.name.clone()).collect();
        assert!(names.contains(&"shell".to_string()));
        assert!(names.contains(&"read".to_string()));
        assert!(names.contains(&"search".to_string()));
        assert!(names.contains(&"respond".to_string()));
    }

    #[test]
    fn test_build_tools_for_context_unknown() {
        let registry = DynamicToolRegistry::new();
        let tools = build_tools_for_context(&registry, "unknown_route");
        let names: Vec<String> = tools.iter().map(|t| t.function.name.clone()).collect();
        assert!(names.contains(&"shell".to_string()));
        assert!(names.contains(&"read".to_string()));
        assert!(names.contains(&"search".to_string()));
        assert!(names.contains(&"respond".to_string()));
    }

    #[test]
    fn test_stable_ordering() {
        let registry = DynamicToolRegistry::new();
        let tools = build_current_tools(&registry);
        let names: Vec<String> = tools.iter().map(|t| t.function.name.clone()).collect();
        let mut sorted_names = names.clone();
        sorted_names.sort();
        assert_eq!(
            names, sorted_names,
            "Tools must be returned in stable alphabetic order"
        );
    }

    #[test]
    fn test_implementation_kind_priority_rust_native_highest() {
        assert!(ImplementationKind::RustNative.selection_priority() > ImplementationKind::Shell.selection_priority());
        assert!(ImplementationKind::RustNative.selection_priority() > ImplementationKind::Network.selection_priority());
        assert!(ImplementationKind::RustWrapper.selection_priority() > ImplementationKind::Shell.selection_priority());
    }

    #[test]
    fn test_offline_capable_tools() {
        assert!(ImplementationKind::RustNative.is_offline_capable());
        assert!(ImplementationKind::RustWrapper.is_offline_capable());
        assert!(ImplementationKind::Shell.is_offline_capable());
        assert!(!ImplementationKind::Network.is_offline_capable());
        assert!(!ImplementationKind::External.is_offline_capable());
    }

    #[test]
    fn test_tool_metadata_read_is_rust_native() {
        let registry = DynamicToolRegistry::new();
        let read = registry.get("read").unwrap();
        assert_eq!(read.implementation_kind, ImplementationKind::RustNative);
        assert!(read.shell_equivalents.contains(&"cat".to_string()));
    }

    #[test]
    fn test_tool_metadata_shell_is_shell_kind() {
        let registry = DynamicToolRegistry::new();
        let shell = registry.get("shell").unwrap();
        assert_eq!(shell.implementation_kind, ImplementationKind::Shell);
    }

    #[test]
    fn test_tool_metadata_fetch_is_network() {
        let registry = DynamicToolRegistry::new();
        let fetch = registry.get("fetch").unwrap();
        assert_eq!(fetch.implementation_kind, ImplementationKind::Network);
        assert!(!fetch.workspace_scoped);
    }

    #[test]
    fn test_tool_metadata_glob_equivalent_to_find() {
        let registry = DynamicToolRegistry::new();
        let glob = registry.get("glob").unwrap();
        assert!(glob.shell_equivalents.contains(&"find".to_string()));
        assert_eq!(glob.implementation_kind, ImplementationKind::RustNative);
    }

    #[test]
    fn test_tool_metadata_edit_equivalent_to_sed() {
        let registry = DynamicToolRegistry::new();
        let edit = registry.get("edit").unwrap();
        assert!(edit.shell_equivalents.contains(&"sed".to_string()));
    }

    #[test]
    fn test_observe_tool_is_registered() {
        let registry = DynamicToolRegistry::new();
        let observe = registry.get("observe").unwrap();
        assert_eq!(observe.function.name, "observe");
    }

    #[test]
    fn test_observe_tool_is_rust_native() {
        let registry = DynamicToolRegistry::new();
        let observe = registry.get("observe").unwrap();
        assert_eq!(observe.implementation_kind, ImplementationKind::RustNative);
    }

    #[test]
    fn test_observe_tool_is_not_deferred() {
        let registry = DynamicToolRegistry::new();
        let observe = registry.get("observe").unwrap();
        assert!(!observe.deferred);
    }

    #[test]
    fn test_observe_tool_is_workspace_scoped() {
        let registry = DynamicToolRegistry::new();
        let observe = registry.get("observe").unwrap();
        assert!(observe.workspace_scoped);
    }

    #[test]
    fn test_observe_tool_has_shell_equivalents() {
        let registry = DynamicToolRegistry::new();
        let observe = registry.get("observe").unwrap();
        assert!(observe.shell_equivalents.contains(&"stat".to_string()));
        assert!(observe.shell_equivalents.contains(&"ls -la".to_string()));
        assert!(observe.shell_equivalents.contains(&"file".to_string()));
    }

    #[test]
    fn test_observe_tool_has_search_hints() {
        let registry = DynamicToolRegistry::new();
        let observe = registry.get("observe").unwrap();
        assert!(observe.search_hints.iter().any(|h| h.contains("metadata")));
    }

    #[test]
    fn test_observe_tool_is_searchable() {
        let registry = DynamicToolRegistry::new();
        let results = registry.search("file metadata");
        assert!(results.iter().any(|t| t.function.name == "observe"));
    }

    #[test]
    fn test_observe_tool_search_by_hint() {
        let registry = DynamicToolRegistry::new();
        let results = registry.search("symlink target");
        assert!(results.iter().any(|t| t.function.name == "observe"));
    }

    #[test]
    fn test_read_tool_has_policy() {
        let registry = DynamicToolRegistry::new();
        let read = registry.get("read").unwrap();
        assert!(read.policy.risks.contains(&ToolRisk::ReadOnly));
        assert!(read.policy.concurrency_safe);
        assert!(!read.policy.requires_permission);
    }

    #[test]
    fn test_shell_tool_has_policy() {
        let registry = DynamicToolRegistry::new();
        let shell = registry.get("shell").unwrap();
        assert!(shell.policy.risks.contains(&ToolRisk::ExternalProcess));
        assert!(!shell.policy.concurrency_safe);
        assert!(shell.policy.requires_permission);
    }

    #[test]
    fn test_edit_tool_has_policy() {
        let registry = DynamicToolRegistry::new();
        let edit = registry.get("edit").unwrap();
        assert!(edit.policy.risks.contains(&ToolRisk::WorkspaceWrite));
        assert!(edit.policy.requires_permission);
        assert!(edit.policy.requires_prior_read);
        assert!(edit.policy.mutates_workspace);
    }
}

/// Risk categories for a tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolRisk {
    /// Tool only reads data, does not modify anything.
    ReadOnly,
    /// Tool writes to workspace files.
    WorkspaceWrite,
    /// Tool executes external processes.
    ExternalProcess,
    /// Tool makes network requests.
    Network,
    /// Tool mutates conversation state.
    ConversationState,
    /// Tool has destructive potential (deletes files, kills processes, etc.).
    DestructivePotential,
}

/// Concurrency and safety properties of a tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExecutorState {
    /// Pure Rust, no external dependencies.
    PureRust,
    /// Rust but depends on system binaries.
    RustWithSystemDependency,
    /// Backed by shell commands.
    ShellBacked,
    /// Makes network requests.
    NetworkBacked,
    /// Runs via extension/plugin.
    ExtensionBacked,
}

/// Default implementation for ExecutorState
impl Default for ExecutorState {
    fn default() -> Self {
        ExecutorState::PureRust
    }
}

/// Policy metadata for a tool.
#[derive(Clone, Debug)]
pub struct ToolPolicy {
    /// Risk categories this tool belongs to.
    pub risks: Vec<ToolRisk>,
    /// How this tool is executed.
    pub executor_state: ExecutorState,
    /// Whether this tool requires user permission before execution.
    pub requires_permission: bool,
    /// Whether this tool requires reading a file before editing it.
    pub requires_prior_read: bool,
    /// Whether this tool is concurrency-safe (can run in parallel with other safe tools).
    pub concurrency_safe: bool,
    /// Whether this tool creates artifacts outside of normal workspace files.
    pub creates_artifacts: bool,
    /// Whether this tool mutates the workspace.
    pub mutates_workspace: bool,
}

impl Default for ToolPolicy {
    fn default() -> Self {
        Self {
            risks: Vec::new(),
            executor_state: ExecutorState::default(),
            requires_permission: false,
            requires_prior_read: false,
            concurrency_safe: true,
            creates_artifacts: false,
            mutates_workspace: false,
        }
    }
}
