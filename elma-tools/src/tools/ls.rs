use crate::registry::{RegistryBuilder, ToolDefinitionExt};

pub(crate) fn register(builder: &mut RegistryBuilder) {
    builder.insert(
        ToolDefinitionExt::new(
            "ls",
            "List files and directories in a given path. Shows a tree view with file sizes and modification times. Skips hidden files and common system/generated directories. Use this to explore unknown directory structures or inspect what files exist in a specific location. Max 1000 entries.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Directory to list (defaults to workspace root)"},
                    "depth": {"type": "integer", "description": "Maximum recursion depth (default: 2, max: 5)"},
                    "ignore": {"type": "array", "items": {"type": "string"}, "description": "Additional glob patterns to exclude"}
                },
                "required": []
            }),
            vec![
                "list files in directory",
                "show directory contents",
                "list directory tree",
                "explore directory structure",
                "what files are in",
                "directory listing",
                "show folder contents",
                "tree view of directory",
            ],
        )
        .not_deferred(),
    );
}
