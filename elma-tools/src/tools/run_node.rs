use crate::registry::{RegistryBuilder, ToolDefinitionExt, ToolRisk, ExecutorState};

pub(crate) fn register(builder: &mut RegistryBuilder) {
    builder.insert(
        ToolDefinitionExt::new(
            "run_node",
            "Execute JavaScript code using the local Node.js interpreter. Code is written to a temp file and executed. Returns stdout, stderr, and exit code.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "code": {"type": "string", "description": "JavaScript code to execute"},
                    "timeout_seconds": {"type": "number", "description": "Timeout in seconds (default: 30)"}
                },
                "required": ["code"]
            }),
            vec!["node", "execute javascript", "run node", "node script", "js code"],
        )
        .with_implementation(crate::registry::ImplementationKind::RustWrapper)
        .with_risks(vec![ToolRisk::WorkspaceWrite])
        .with_executor_state(ExecutorState::PureRust)
        .concurrency_safe(true),
    );
}
