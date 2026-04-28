use crate::registry::{RegistryBuilder, ToolDefinitionExt};

pub(crate) fn register(builder: &mut RegistryBuilder) {
    builder.insert(
        ToolDefinitionExt::new(
            "shell",
            "Execute a shell command and return its output. Use this to list files, get the current time, run builds, inspect git status, or perform any system operation.",
            serde_json::json!({
                "type": "object",
                "properties": {"command": {"type": "string", "description": "The shell command to execute (e.g. 'ls docs/', 'date', 'cargo build')"}},
                "required": ["command"]
            }),
            vec![
                "execute shell command",
                "run command line",
                "execute bash command",
                "run terminal command",
                "execute system command",
                "list directory files",
                "get current time date",
                "run build test",
            ],
        )
        .not_deferred()
        .with_check_fn(|| {
            which::which("sh").is_ok() || which::which("bash").is_ok()
        }),
    );
}
