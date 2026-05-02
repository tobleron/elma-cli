use crate::registry::{RegistryBuilder, ToolDefinitionExt, ToolRisk, ExecutorState};

pub(crate) fn register(builder: &mut RegistryBuilder) {
    builder.insert(
        ToolDefinitionExt::new(
            "mkdir",
            "Create one or more directories. Creates parent directories if they don't exist.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Path to directory to create"},
                    "parents": {"type": "boolean", "description": "Create parent directories if needed", "default": true}
                },
                "required": ["path"]
            }),
            vec!["create directory", "make directory", "new folder"],
        )
        .not_deferred()
        .with_implementation(crate::registry::ImplementationKind::RustWrapper)
        .with_risks(vec![ToolRisk::WorkspaceWrite])
        .with_executor_state(ExecutorState::PureRust)
        .concurrency_safe(true),
    );
}