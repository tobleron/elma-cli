use crate::registry::{RegistryBuilder, ToolDefinitionExt, ToolRisk, ExecutorState};

pub(crate) fn register(builder: &mut RegistryBuilder) {
    builder.insert(
        ToolDefinitionExt::new(
            "stat",
            "Get file or directory metadata (size, mtime, permissions, type). Use this to inspect file properties without reading content.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Path to file or directory"}
                },
                "required": ["path"]
            }),
            vec!["file metadata", "file info", "file permissions", "file size", "file modified"],
        )
        .not_deferred()
        .with_implementation(crate::registry::ImplementationKind::RustWrapper)
        .with_risks(vec![ToolRisk::ReadOnly])
        .with_executor_state(ExecutorState::PureRust)
        .concurrency_safe(true),
    );
}