use crate::registry::{RegistryBuilder, ToolDefinitionExt, ToolRisk, ExecutorState};

pub(crate) fn register(builder: &mut RegistryBuilder) {
    builder.insert(
        ToolDefinitionExt::new(
            "job_stop",
            "Stop a running background job. Sends a kill signal to terminate the job. Use this to cancel long-running jobs.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "job_id": {"type": "string", "description": "Job ID returned by job_start"}
                },
                "required": ["job_id"]
            }),
            vec!["job stop", "stop job", "cancel job", "kill background job"],
        )
        .with_implementation(crate::registry::ImplementationKind::RustWrapper)
        .with_risks(vec![ToolRisk::WorkspaceWrite])
        .with_executor_state(ExecutorState::PureRust)
        .concurrency_safe(true),
    );
}
