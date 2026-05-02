use crate::registry::{ExecutorState, RegistryBuilder, ToolDefinitionExt, ToolRisk};

pub(crate) fn register(builder: &mut RegistryBuilder) {
    builder.insert(
        ToolDefinitionExt::new(
            "workspace_info",
            "Get information about the current workspace: root path, directory structure, project type (Cargo.toml, package.json, etc.), git status, and any active project guidance documents (AGENTS.md, _tasks/TASKS.md). Use this to understand where you are and what kind of project you're working in. Call this early when you need to read files, run commands, or understand the project structure.",
            serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
            vec![
                "workspace",
                "project info",
                "what directory am I in",
                "project structure",
                "repo info",
                "working directory",
                "current project",
                "project type",
            ],
        )
        .not_deferred()
        .with_implementation(crate::registry::ImplementationKind::RustNative)
        .with_risks(vec![ToolRisk::ReadOnly])
        .with_executor_state(ExecutorState::PureRust)
        .concurrency_safe(true),
    );
}
