use crate::registry::{RegistryBuilder, ToolDefinitionExt, ToolRisk, ExecutorState};

pub(crate) fn register(builder: &mut RegistryBuilder) {
    builder.insert(
        ToolDefinitionExt::new(
            "read",
            "Read the contents of one or more files. ALWAYS use this instead of 'shell cat'. It handles source code, documents, config files, PDFs, and EPUBs natively. It can read multiple files at once efficiently.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Workspace-relative path to the file to read"},
                    "paths": {"type": "array", "items": {"type": "string"}, "description": "Multiple workspace-relative paths to read in one call, preserving order with per-file headers"}
                },
                "oneOf": [
                    {"required": ["path"]},
                    {"required": ["paths"]}
                ]
            }),
            vec![
                "read file contents",
                "open file for reading",
                "view file content",
                "display file contents",
                "read source code",
                "read document",
                "read multiple files",
                "cat command equivalent",
                "read pdf epub document",
            ],
        )
        .not_deferred()
        .with_implementation(crate::registry::ImplementationKind::RustNative)
        .with_shell_equivalents(vec!["cat", "head", "tail", "less"])
        .with_risks(vec![ToolRisk::ReadOnly])
        .with_executor_state(ExecutorState::PureRust)
        .concurrency_safe(true),
    );
}
