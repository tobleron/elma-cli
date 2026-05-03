use crate::registry::{RegistryBuilder, ToolDefinitionExt, ToolRisk, ExecutorState};

pub(crate) fn register(builder: &mut RegistryBuilder) {
    builder.insert(
        ToolDefinitionExt::new(
            "shell",
            "Execute a shell command and return its output. ONLY use this for operations without a native tool equivalent (e.g. running builds, git status, date). NEVER use for listing files, reading files, or searching text—use 'ls', 'read', or 'search' instead.",
            serde_json::json!({
                "type": "object",
                "properties": {"command": {"type": "string", "description": "The shell command to execute (e.g. 'cargo build', 'git status', 'date')"}},
                "required": ["command"]
            }),
            vec![
                "execute shell command",
                "run command line",
                "execute bash command",
                "run terminal command",
                "execute system command",
                "get current time date",
                "run build test",
            ],
        )
        .not_deferred()
        .with_implementation(crate::registry::ImplementationKind::Shell)
        .with_shell_equivalents(vec!["sh", "bash", "zsh"])
        .with_check_fn(|| {
            which::which("sh").is_ok() || which::which("bash").is_ok()
        })
        .with_risks(vec![ToolRisk::ExternalProcess])
        .with_executor_state(ExecutorState::ShellBacked)
        .requires_permission(true)
        .concurrency_safe(false)
        .mutates_workspace(true),
    );
}
