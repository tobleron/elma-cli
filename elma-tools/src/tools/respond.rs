use crate::registry::{RegistryBuilder, ToolDefinitionExt, ToolRisk, ExecutorState};

pub(crate) fn register(builder: &mut RegistryBuilder) {
    builder.insert(
        ToolDefinitionExt::new(
            "respond",
            "Provide your final answer to the user. Use this when you have gathered enough evidence or the conversation is complete. The tool loop will stop after this call — your answer will be delivered directly to the user.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "answer": {"type": "string"},
                    "content": {"type": "string"},
                    "text": {"type": "string"}
                },
                "anyOf": [
                    {"required": ["answer"]},
                    {"required": ["content"]},
                    {"required": ["text"]}
                ]
            }),
            vec![
                "provide final answer",
                "respond to user",
                "give answer to user",
            ],
        )
        .not_deferred()
        .with_implementation(crate::registry::ImplementationKind::RustNative)
        .with_risks(vec![ToolRisk::ConversationState])
        .with_executor_state(ExecutorState::PureRust)
        .concurrency_safe(false),
    );
}
