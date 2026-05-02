use crate::registry::{RegistryBuilder, ToolDefinitionExt, ToolRisk, ExecutorState};

pub(crate) fn register(builder: &mut RegistryBuilder) {
    builder.insert(
        ToolDefinitionExt::new(
            "git_inspect",
            "Inspect git repository state without using shell. Returns structured information about status, branches, changes, commits. Modes: status, branch, changed_files, diffstat, recent_commits.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "mode": {
                        "type": "string",
                        "description": "Inspection mode",
                        "enum": ["status", "branch", "changed_files", "diffstat", "recent_commits"]
                    },
                    "path": {"type": "string", "description": "Optional path scope (default: repo root)"}
                },
                "required": ["mode"]
            }),
            vec!["git", "git status", "git branch", "git diff", "repository state", "git inspect"],
        )
        .with_implementation(crate::registry::ImplementationKind::RustWrapper)
        .with_risks(vec![ToolRisk::ReadOnly])
        .with_executor_state(ExecutorState::PureRust)
        .concurrency_safe(true),
    );
}
