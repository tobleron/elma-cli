use crate::registry::{RegistryBuilder, ToolDefinitionExt, ToolRisk, ExecutorState};

pub(crate) fn register(builder: &mut RegistryBuilder) {
    builder.insert(
        ToolDefinitionExt::new(
            "exists",
            "Quickly check if one or more paths exist in the workspace. This is the fastest way to verify a path without reading content. Supports multiple paths.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Single workspace-relative path to check"},
                    "paths": {"type": "array", "items": {"type": "string"}, "description": "Multiple workspace-relative paths to check simultaneously"}
                },
                "oneOf": [
                    {"required": ["path"]},
                    {"required": ["paths"]}
                ]
            }),
            vec![
                "check if file exists",
                "path existence check",
                "is file there",
                "verify path",
                "exists check",
                "check multiple files exist",
                "fast path verification",
            ],
        )
        .not_deferred()
        .with_implementation(crate::registry::ImplementationKind::RustWrapper)
        .with_risks(vec![ToolRisk::ReadOnly])
        .with_executor_state(ExecutorState::PureRust)
        .concurrency_safe(true),
    );
}