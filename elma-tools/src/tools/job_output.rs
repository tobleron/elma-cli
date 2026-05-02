use crate::registry::{RegistryBuilder, ToolDefinitionExt, ToolRisk, ExecutorState};

pub(crate) fn register(builder: &mut RegistryBuilder) {
    builder.insert(
        ToolDefinitionExt::new(
            "job_output",
            "Get the output (stdout/stderr) of a background job. Returns buffered output lines. Use job_status first to check if the job is complete.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "job_id": {"type": "string", "description": "Job ID returned by job_start"}
                },
                "required": ["job_id"]
            }),
            vec!["job output", "background job output", "get job output", "job stdout"],
        )
        .with_implementation(crate::registry::ImplementationKind::RustWrapper)
        .with_risks(vec![ToolRisk::ReadOnly])
        .with_executor_state(ExecutorState::PureRust)
        .concurrency_safe(true),
    );
}
