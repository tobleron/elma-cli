use crate::registry::{RegistryBuilder, ToolDefinitionExt, ToolPolicy, ToolRisk, ExecutorState};

pub(crate) fn register(builder: &mut RegistryBuilder) {
    builder.insert(
        ToolDefinitionExt {
            tool_type: "function".to_string(),
            function: crate::types::ToolFunction {
                name: "tool_search".to_string(),
                description: "Search for additional or extension tools by capability. The core tools (shell, read, search, respond, update_todo_list) are always available — use this only to discover specialty tools beyond the core set.".to_string(),
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
            deferred: false,
            implementation_kind: crate::registry::ImplementationKind::RustNative,
            workspace_scoped: true,
            shell_equivalents: Vec::new(),
            check_fn: None,
            policy: ToolPolicy {
                risks: vec![ToolRisk::ReadOnly],
                executor_state: ExecutorState::PureRust,
                requires_permission: false,
                requires_prior_read: false,
                concurrency_safe: true,
                creates_artifacts: false,
                mutates_workspace: false,
            },
        },
    );
}
