use crate::registry::{RegistryBuilder, ToolDefinitionExt};

pub(crate) fn register(builder: &mut RegistryBuilder) {
    builder.insert(
        ToolDefinitionExt::new(
            "edit",
            "Edit a file by exact find-and-replace. The old_string must match exactly (including whitespace, indentation, and blank lines). If old_string is found multiple times, the edit is blocked — include more surrounding context to make it unique. Cannot create or delete content — use write tool for new files. Prefer editing existing files in the codebase.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string", "description": "Absolute path to the file to modify"},
                    "old_string": {"type": "string", "description": "The exact text to replace — must include all whitespace, indentation, and surrounding context"},
                    "new_string": {"type": "string", "description": "The text to replace it with"},
                    "replace_all": {"type": "boolean", "description": "Replace all occurrences of old_string (default false)"}
                },
                "required": ["file_path", "old_string", "new_string"]
            }),
            vec![
                "edit file",
                "find and replace",
                "modify file",
                "replace text",
                "update source code",
                "fix typo in file",
                "refactor code",
                "rename symbol in file",
            ],
        )
        .not_deferred()
        .with_implementation(crate::registry::ImplementationKind::RustNative)
        .with_shell_equivalents(vec!["sed", "awk", "perl -i"]),
    );
}
