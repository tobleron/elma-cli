use crate::registry::{RegistryBuilder, ToolDefinitionExt, ToolRisk, ExecutorState};

pub(crate) fn register(builder: &mut RegistryBuilder) {
    builder.insert(
        ToolDefinitionExt::new(
            "job_start",
            "Start a long-running command in the background. Returns a job ID to check status, output, or stop the job. The job runs without blocking the session.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {"type": "string", "description": "Shell command to run in background"},
                    "name": {"type": "string", "description": "Optional name for the job"},
                    "memory_limit_mb": {"type": "number", "description": "Memory limit in MB (default: 2048)"},
                    "timeout_seconds": {"type": "number", "description": "Timeout in seconds (default: 300)"}
                },
                "required": ["command"]
            }),
            vec!["background job", "start job", "run in background", "long running command"],
        )
        .with_implementation(crate::registry::ImplementationKind::RustWrapper)
        .with_risks(vec![ToolRisk::WorkspaceWrite])
        .with_executor_state(ExecutorState::PureRust)
        .concurrency_safe(true),
    );
}
