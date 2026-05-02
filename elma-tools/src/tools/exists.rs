use crate::registry::{RegistryBuilder, ToolDefinitionExt, ToolRisk, ExecutorState};

pub(crate) fn register(builder: &mut RegistryBuilder) {
    builder.insert(
        ToolDefinitionExt::new(
            "exists",
            "Check if a path exists and its type (file, directory, or other).",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Path to check"},
                    "type": {"type": "string", "description": "Optional: 'file', 'dir', or 'any'", "default": "any"}
                },
                "required": ["path"]
            }),
            vec!["check path exists", "path exists", "file exists", "directory exists"],
        )
        .not_deferred()
        .with_implementation(crate::registry::ImplementationKind::RustWrapper)
        .with_risks(vec![ToolRisk::ReadOnly])
        .with_executor_state(ExecutorState::PureRust)
        .concurrency_safe(true),
    );
}