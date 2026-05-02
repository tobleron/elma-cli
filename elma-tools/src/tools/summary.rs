use crate::registry::{RegistryBuilder, ToolDefinitionExt, ToolRisk, ExecutorState};

pub(crate) fn register(builder: &mut RegistryBuilder) {
    builder.insert(
        ToolDefinitionExt::new(
            "summary",
            "Provide a short final summary when a MULTI-STEP task is COMPLETE. Use this only when you have gathered evidence, executed tools, and fully resolved the request. Do NOT use for simple greetings or conversational exchanges — use respond instead for those. This stops the tool loop.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "content": {
                        "type": "string",
                        "description": "Short final summary/answer (be concise)"
                    }
                },
                "required": ["content"]
            }),
            vec![
                "provide final summary",
                "task complete",
                "give final answer",
                "request resolved",
            ],
        )
        .not_deferred()
        .with_implementation(crate::registry::ImplementationKind::RustNative)
        .with_risks(vec![ToolRisk::ConversationState])
        .with_executor_state(ExecutorState::PureRust)
        .concurrency_safe(false),
    );
}
