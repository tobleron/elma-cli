use crate::registry::{RegistryBuilder, ToolDefinitionExt};

pub(crate) fn register(builder: &mut RegistryBuilder) {
    builder.insert(
        ToolDefinitionExt::new(
            "respond",
            "Provide a final answer to the user.",
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
        .with_implementation(crate::registry::ImplementationKind::RustNative),
    );
}
