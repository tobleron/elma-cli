use crate::registry::{RegistryBuilder, ToolDefinitionExt};

pub(crate) fn register(builder: &mut RegistryBuilder) {
    builder.insert(
        ToolDefinitionExt::new(
            "update_todo_list",
            "Create and update a local task/todo list for multi-step work. Use this to track progress when handling requests with multiple steps.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {"type":"string","enum":["add","update","in_progress","completed","blocked","remove","list"]},
                    "id": {"type":"integer"},
                    "text": {"type":"string"},
                    "reason": {"type":"string"}
                },
                "required": ["action"]
            }),
            vec![
                "manage todo list",
                "create task list",
                "update task status",
                "track tasks",
                "multi-step task tracking",
            ],
        )
        .not_deferred()
        .with_implementation(crate::registry::ImplementationKind::RustNative),
    );
}
