use crate::registry::{RegistryBuilder, ToolDefinitionExt, ToolRisk, ExecutorState};

pub(crate) fn register(builder: &mut RegistryBuilder) {
    builder.insert(
        ToolDefinitionExt::new(
            "trash",
            "Move a file or directory to trash rather than permanently deleting. Safer than rm.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Path to move to trash"}
                },
                "required": ["path"]
            }),
            vec!["delete file", "remove file", "move to trash", "safe delete"],
        )
        .not_deferred()
        .with_implementation(crate::registry::ImplementationKind::RustWrapper)
        .with_risks(vec![ToolRisk::WorkspaceWrite])
        .with_executor_state(ExecutorState::PureRust)
        .concurrency_safe(false),
    );
}