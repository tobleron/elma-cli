use crate::registry::{RegistryBuilder, ToolDefinitionExt, ToolRisk, ExecutorState};

pub(crate) fn register(builder: &mut RegistryBuilder) {
    builder.insert(
        ToolDefinitionExt::new(
            "write",
            "Create or overwrite a file with given content. ALWAYS use this instead of 'shell tee'. Auto-creates parent directories. Use this for creating new files or complete rewrites. For surgical changes to existing files, use the 'edit' tool instead. NOTE: Writing a .rs file will trigger an automatic 'cargo check' verification.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Workspace-relative path to the file"},
                    "content": {"type": "string", "description": "The content to write to the file"}
                },
                "required": ["path", "content"]
            }),
            vec![
                "write file",
                "create new file",
                "overwrite file",
                "save content to file",
                "create file with content",
                "write output to file",
            ],
        )
        .not_deferred()
        .with_implementation(crate::registry::ImplementationKind::RustNative)
        .with_shell_equivalents(vec!["tee", "cp", "dd"])
        .with_risks(vec![ToolRisk::WorkspaceWrite])
        .with_executor_state(ExecutorState::PureRust)
        .requires_permission(true)
        .requires_prior_read(false)
        .concurrency_safe(false)
        .mutates_workspace(true),
    );
}
