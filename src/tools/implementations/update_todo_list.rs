use crate::tools::types::{ToolExecutionResult};

pub fn exec_update_todo_list(
    av: &serde_json::Value,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let action = av["action"].as_str().unwrap_or("").trim().to_string();
    if action.is_empty() {
        let error_msg = "Error: action is required";
        return ToolExecutionResult::new_failed(call_id, "update_todo_list", error_msg);
    }
    let id = av["id"].as_u64().map(|v| v as u32);
    let text = av["text"].as_str().map(|s| s.to_string());
    let reason = av["reason"].as_str().map(|s| s.to_string());

    let content = match (action.as_str(), tui.as_mut()) {
        ("add", Some(t)) => {
            let desc = text.unwrap_or_else(|| "New task".to_string());
            let new_id = t.todo_add(desc.clone());
            format!("Added task {}: {}", new_id, desc)
        }
        ("update", Some(t)) => {
            if let (Some(id), Some(text)) = (id, text) {
                t.todo_update(id, text.clone());
                format!("Updated task {}: {}", id, text)
            } else {
                "Error: update requires id and text".to_string()
            }
        }
        ("in_progress", Some(t)) => {
            if let Some(id) = id {
                t.todo_start(id);
                format!("Task {} marked in progress", id)
            } else {
                "Error: in_progress requires id".to_string()
            }
        }
        ("completed", Some(t)) => {
            if let Some(id) = id {
                t.todo_complete(id);
                format!("Task {} marked completed", id)
            } else {
                "Error: completed requires id".to_string()
            }
        }
        ("blocked", Some(t)) => {
            if let Some(id) = id {
                t.todo_block(id, reason.clone());
                if let Some(r) = reason {
                    format!("Task {} blocked: {}", id, r)
                } else {
                    format!("Task {} blocked", id)
                }
            } else {
                "Error: blocked requires id".to_string()
            }
        }
        ("remove", Some(t)) => {
            if let Some(id) = id {
                if t.todo_remove(id) {
                    format!("Removed task {}", id)
                } else {
                    format!("Task {} not found", id)
                }
            } else {
                "Error: remove requires id".to_string()
            }
        }
        ("list", Some(t)) => {
            let lines = t.todo_render_lines();
            if lines.is_empty() {
                "No tasks".to_string()
            } else {
                lines.join("\n")
            }
        }
        (_, None) => "Todo updates require interactive TUI mode".to_string(),
        _ => format!("Unknown action: {}", action),
    };

    let ok = !content.starts_with("Error:");

    if let Some(t) = tui.as_mut() {
        t.add_claude_message(crate::claude_ui::ClaudeMessage::ToolResult {
            name: "update_todo_list".to_string(),
            success: ok,
            output: content.clone(),
            duration_ms: None,
        });
    }

    if ok {
        ToolExecutionResult::new_ok(call_id, "update_todo_list", &content)
    } else {
        ToolExecutionResult::new_failed(call_id, "update_todo_list", &content)
    }
}
