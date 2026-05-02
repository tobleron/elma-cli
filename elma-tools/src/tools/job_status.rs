use crate::registry::{RegistryBuilder, ToolDefinitionExt, ToolRisk, ExecutorState};

pub(crate) fn register(builder: &mut RegistryBuilder) {
    builder.insert(
        ToolDefinitionExt::new(
            "job_status",
            "Check the status of a background job. Returns status (pending/running/completed/failed), exit code, runtime, and memory usage.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "job_id": {"type": "string", "description": "Job ID returned by job_start"}
                },
                "required": ["job_id"]
            }),
            vec!["job status", "background job status", "check job", "is job running"],
        )
        .with_implementation(crate::registry::ImplementationKind::RustWrapper)
        .with_risks(vec![ToolRisk::ReadOnly])
        .with_executor_state(ExecutorState::PureRust)
        .concurrency_safe(true),
    );
}
