use crate::registry::{RegistryBuilder, ToolDefinitionExt};

pub(crate) fn register(builder: &mut RegistryBuilder) {
    builder.insert(
        ToolDefinitionExt::new(
            "read_evidence",
            "Retrieve full raw evidence content by evidence ID. Use when compact summaries in the narrative are insufficient. Evidence IDs look like 'e_001', 'e_002', etc.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "ids": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "List of evidence IDs to retrieve (e.g., [\"e_001\", \"e_002\"])"
                    }
                },
                "required": ["ids"]
            }),
            vec![
                "read evidence content",
                "retrieve raw evidence",
                "get full tool output",
                "access evidence ledger",
            ],
        )
        .not_deferred(),
    );
}
