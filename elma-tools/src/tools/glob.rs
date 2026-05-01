use crate::registry::{RegistryBuilder, ToolDefinitionExt};

pub(crate) fn register(builder: &mut RegistryBuilder) {
    builder.insert(
        ToolDefinitionExt::new(
            "glob",
            "Find files matching a glob pattern. Use for filename-based search (e.g., '**/*.rs', 'src/**/mod.rs', '*.toml'). Returns relative file paths sorted by modification time. Respects .gitignore. Max 100 results.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {"type": "string", "description": "The glob pattern to match (e.g., '**/*.rs', 'src/**/*.toml', '*GEMINI*')"},
                    "path": {"type": "string", "description": "Directory to search in (defaults to workspace root)"}
                },
                "required": ["pattern"]
            }),
            vec![
                "find files by name",
                "search filename pattern",
                "list files matching pattern",
                "glob search files",
                "find file by name",
                "locate file",
                "file pattern matching",
            ],
        )
        .not_deferred()
        .with_implementation(crate::registry::ImplementationKind::RustNative)
        .with_shell_equivalents(vec!["find", "fd"]),
    );
}
