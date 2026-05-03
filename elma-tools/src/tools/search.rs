use crate::registry::{RegistryBuilder, ToolDefinitionExt, ToolRisk, ExecutorState};

pub(crate) fn register(builder: &mut RegistryBuilder) {
    builder.insert(
        ToolDefinitionExt::new(
            "search",
            "Search for a pattern in the workspace. ALWAYS use this instead of 'shell grep' or 'shell rg'. It is optimized for codebase search and supports glob filtering and multi-path scope. Max 50 matches.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {"type": "string", "description": "The search pattern (plain text or regex)"},
                    "path": {"type": "string", "description": "Optional subdirectory to search in"},
                    "paths": {"type": "array", "items": {"type": "string"}, "description": "Optional multiple directories to search in"},
                    "includes": {"type": "array", "items": {"type": "string"}, "description": "Optional glob patterns to include (e.g. ['*.rs'])"}
                },
                "required": ["pattern"]
            }),
            vec![
                "search for text",
                "find pattern in files",
                "grep in workspace",
                "search codebase",
                "find string",
                "ripgrep equivalent",
                "grep command equivalent",
                "search in multiple paths",
            ],
        )
        .not_deferred()
        .with_implementation(crate::registry::ImplementationKind::RustWrapper)
        .with_shell_equivalents(vec!["grep", "rg", "ag", "ack"])
        .with_check_fn(|| {
            which::which("rg").is_ok() || which::which("grep").is_ok()
        })
        .with_risks(vec![ToolRisk::ReadOnly])
        .with_executor_state(ExecutorState::RustWithSystemDependency)
        .concurrency_safe(true),
    );
}
