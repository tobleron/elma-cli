use crate::registry::{RegistryBuilder, ToolDefinitionExt};

pub(crate) fn register(builder: &mut RegistryBuilder) {
    builder.insert(
        ToolDefinitionExt::new(
            "search",
            "Search for text patterns in files using ripgrep. Use this to find function definitions, usages, config keys, or any text across the workspace. Set literal_text=true to search for literal text without regex escaping.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {"type": "string", "description": "The text or regex pattern to search for"},
                    "path": {"type": "string", "description": "Optional directory or file path to restrict the search scope"},
                    "literal_text": {"type": "boolean", "description": "If true, treat pattern as literal text (auto-escapes regex special chars)"},
                    "include": {"type": "string", "description": "File pattern to filter by (e.g. '*.rs', '*.{ts,tsx}')"}
                },
                "required": ["pattern"]
            }),
            vec![
                "search text in files",
                "find pattern in code",
                "grep search files",
                "search file contents",
                "find text pattern",
                "find function definition",
                "search workspace code",
            ],
        )
        .not_deferred()
        .with_check_fn(|| {
            which::which("rg").is_ok() || which::which("grep").is_ok()
        }),
    );
}
