use crate::registry::{ExecutorState, RegistryBuilder, ToolDefinitionExt, ToolRisk};

pub(crate) fn register(builder: &mut RegistryBuilder) {
    builder.insert(
        ToolDefinitionExt::new(
            "backup",
            "Create a safe backup of source files preserving directory hierarchy. \
             Walks source directory, copies matching files to destination, writes a manifest, \
             and verifies file counts. Automatically excludes .git, target, node_modules, \
             .trash, sessions, and project_tmp. Supports include patterns (glob) and extra excludes.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "source_dir": {
                        "type": "string",
                        "description": "Source directory to back up (relative to workspace root)"
                    },
                    "dest_dir": {
                        "type": "string",
                        "description": "Destination directory for the backup"
                    },
                    "include_patterns": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Glob patterns to include (e.g. ['**/*.rs', '**/*.toml'])"
                    },
                    "exclude_patterns": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Extra patterns to exclude beyond defaults"
                    },
                    "verify": {
                        "type": "boolean",
                        "description": "Whether to verify source vs copied file counts (default true)"
                    }
                },
                "required": ["source_dir", "dest_dir"]
            }),
            vec!["backup", "safe backup", "backup files", "copy with manifest"],
        )
        .not_deferred()
        .with_implementation(crate::registry::ImplementationKind::RustWrapper)
        .with_risks(vec![ToolRisk::WorkspaceWrite])
        .with_executor_state(ExecutorState::PureRust)
        .concurrency_safe(false),
    );
}
