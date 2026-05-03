use crate::registry::{RegistryBuilder, ToolDefinitionExt, ToolRisk, ExecutorState};

pub(crate) fn register(builder: &mut RegistryBuilder) {
    builder.insert(
        ToolDefinitionExt::new(
            "mkdir",
            "Create a directory. ALWAYS use this instead of 'shell mkdir'. It auto-creates parent directories by default. Returns success if directory already exists.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Workspace-relative path to directory to create"},
                    "parents": {"type": "boolean", "description": "Create parent directories if needed", "default": true}
                },
                "required": ["path"]
            }),
            vec!["create directory", "make directory", "new folder", "mkdir"],
        )
        .not_deferred()
        .with_implementation(crate::registry::ImplementationKind::RustWrapper)
        .with_risks(vec![ToolRisk::WorkspaceWrite])
        .with_executor_state(ExecutorState::PureRust)
        .concurrency_safe(true),
    );
}