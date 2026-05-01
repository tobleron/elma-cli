use crate::registry::{RegistryBuilder, ToolDefinitionExt, ToolRisk};

pub(crate) fn register(builder: &mut RegistryBuilder) {
    builder.insert(
        ToolDefinitionExt::new(
            "read",
            "Read the contents of a file (source code, documents, config files, PDFs, EPUBs). Prefer this over shell cat for structured document types.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Absolute or workspace-relative path to the file to read"},
                    "paths": {"type": "array", "items": {"type": "string"}, "description": "Multiple file paths to read in one call, preserving order with per-file headers"}
                },
                "oneOf": [
                    {"required": ["path"]},
                    {"required": ["paths"]}
                ]
            }),
            vec![
                "read file contents",
                "open file for reading",
                "view file content",
                "display file contents",
                "read source code",
                "read document",
                "read multiple files",
            ],
        )
        .not_deferred()
        .with_risks(vec![ToolRisk::ReadOnly])
        .concurrency_safe(),
    );
}
