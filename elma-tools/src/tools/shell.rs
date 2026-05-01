use crate::registry::{RegistryBuilder, ToolDefinitionExt, ToolRisk};

pub(crate) fn register(builder: &mut RegistryBuilder) {
    builder.insert(
        ToolDefinitionExt::new(
            "shell",
            "Execute a shell command and return its output.\n\nExecution environment notes:\n- Commands run under a clean /bin/sh session (not an interactive/login shell).\n- Prefer POSIX sh syntax for portability.\n- Avoid bash-4+ only features like `mapfile` and associative arrays, and avoid process substitution.\n\nUse this to list files, run builds/tests, inspect git status, or perform other system operations.",
            serde_json::json!({
                "type": "object",
                "properties": {"command": {"type": "string", "description": "The shell command to execute (e.g. 'ls docs/', 'date', 'cargo build')"}},
                "required": ["command"]
            }),
            vec![
                "execute shell command",
                "run command line",
                "run terminal command",
                "execute system command",
                "list directory files",
                "get current time date",
                "run build test",
            ],
        )
        .not_deferred()
        .with_risks(vec![ToolRisk::ExternalProcess])
        .requires_permission()
        .with_check_fn(|| {
            which::which("sh").is_ok() || which::which("bash").is_ok()
        }),
    );
}
