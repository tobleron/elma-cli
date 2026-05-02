use crate::registry::{RegistryBuilder, ToolDefinitionExt, ToolRisk, ExecutorState};

pub(crate) fn register(builder: &mut RegistryBuilder) {
    builder.insert(
        ToolDefinitionExt::new(
            "observe",
            "Inspect file or directory metadata without reading contents. Returns path existence, file type, size, modification time, directory child count, and symlink target. Use before read when you only need metadata — avoids wasting context on full file contents.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Workspace-relative path to inspect"}
                },
                "required": ["path"]
            }),
            vec![
                "check file existence",
                "file metadata inspection",
                "get file size and type",
                "inspect directory contents count",
                "check if path exists",
                "symlink target inspection",
                "file stats without reading",
                "metadata-only file check",
                "lightweight path inspection",
            ],
        )
        .not_deferred()
        .with_implementation(crate::registry::ImplementationKind::RustNative)
        .with_shell_equivalents(vec!["stat", "ls -la", "file"])
        .with_risks(vec![ToolRisk::ReadOnly])
        .with_executor_state(ExecutorState::PureRust)
        .concurrency_safe(true),
    );
}
