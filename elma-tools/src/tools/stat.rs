use crate::registry::{RegistryBuilder, ToolDefinitionExt, ToolRisk, ExecutorState};

pub(crate) fn register(builder: &mut RegistryBuilder) {
    builder.insert(
        ToolDefinitionExt::new(
            "stat",
            "Get file or directory metadata (size, mtime, permissions, type). ALWAYS use this instead of 'shell stat'. It is the fastest way to inspect file properties without reading content.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Workspace-relative path to file or directory"}
                },
                "required": ["path"]
            }),
            vec!["file metadata", "file info", "file permissions", "file size", "file modified", "stat command equivalent"],
        )
        .not_deferred()
        .with_implementation(crate::registry::ImplementationKind::RustWrapper)
        .with_risks(vec![ToolRisk::ReadOnly])
        .with_executor_state(ExecutorState::PureRust)
        .concurrency_safe(true),
    );
}