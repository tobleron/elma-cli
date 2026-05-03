use crate::registry::{RegistryBuilder, ToolDefinitionExt, ToolRisk, ExecutorState};

pub(crate) fn register(builder: &mut RegistryBuilder) {
    builder.insert(
        ToolDefinitionExt::new(
            "ls",
            "List files and directories in a given path. ALWAYS use this instead of 'shell ls' or 'shell tree'. It returns a superior tree view with file sizes and modification times. Use this to explore unknown directory structures or inspect what files exist in a specific location. Max 1000 entries.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Directory to list (defaults to workspace root). Use '.' for current dir."},
                    "depth": {"type": "integer", "description": "Maximum recursion depth (default: 2, max: 5)"},
                    "ignore": {"type": "array", "items": {"type": "string"}, "description": "Additional glob patterns to exclude"}
                },
                "required": []
            }),
            vec![
                "list files in directory",
                "show directory contents",
                "list directory tree",
                "explore directory structure",
                "what files are in",
                "directory listing",
                "show folder contents",
                "tree view of directory",
                "ls command equivalent",
                "tree command equivalent",
            ],
        )
        .not_deferred()
        .with_implementation(crate::registry::ImplementationKind::RustNative)
        .with_shell_equivalents(vec!["ls", "tree"])
        .with_risks(vec![ToolRisk::ReadOnly])
        .with_executor_state(ExecutorState::PureRust)
        .concurrency_safe(true),
    );
}
