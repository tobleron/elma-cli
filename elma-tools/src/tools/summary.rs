use crate::registry::{RegistryBuilder, ToolDefinitionExt};

pub(crate) fn register(builder: &mut RegistryBuilder) {
    builder.insert(
        ToolDefinitionExt::new(
            "summary",
            "Provide a final short summary when the task is COMPLETE. Use this only when you have fully answered the question or completed the request. Keep it brief and concise. This stops the tool loop.",
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
        .not_deferred(),
    );
}
