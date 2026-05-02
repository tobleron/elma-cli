use crate::registry::{RegistryBuilder, ToolDefinitionExt, ToolRisk, ExecutorState};

pub(crate) fn register(builder: &mut RegistryBuilder) {
    builder.insert(
        ToolDefinitionExt::new(
            "repo_map",
            "Generate a token-budgeted repository map showing symbols (functions, structs, classes) for code files. Uses cache to avoid re-scanning unchanged files.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "token_budget": {"type": "number", "description": "Max tokens for output (default: 2000)"},
                    "max_files": {"type": "number", "description": "Max files to process (default: 50)"}
                },
                "required": []
            }),
            vec!["repo map", "symbol map", "code map", "repository structure", "symbols"],
        )
        .with_implementation(crate::registry::ImplementationKind::RustWrapper)
        .with_risks(vec![ToolRisk::ReadOnly])
        .with_executor_state(ExecutorState::PureRust)
        .concurrency_safe(true),
    );
}
