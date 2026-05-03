//! @efficiency-role: util-pure
//! Shared helper functions for tool execution.

use std::path::PathBuf;

pub fn emit_tool_progress(
    _tui: &mut Option<&mut crate::ui_terminal::TerminalUI>,
    _name: &str,
    _message: &str,
) {
}

pub fn emit_tool_start(tui: &mut Option<&mut crate::ui_terminal::TerminalUI>, name: &str, input: &str) {
    if let Some(t) = tui.as_mut() {
        t.handle_ui_event(crate::claude_ui::UiEvent::ToolStarted {
            name: name.to_string(),
            command: input.to_string(),
        });
    }
}

pub fn emit_tool_result(
    tui: &mut Option<&mut crate::ui_terminal::TerminalUI>,
    name: &str,
    success: bool,
    output: &str,
) {
    if let Some(t) = tui.as_mut() {
        t.handle_ui_event(crate::claude_ui::UiEvent::ToolFinished {
            name: name.to_string(),
            success,
            output: output.to_string(),
        });
    }
}

pub fn format_time(secs: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let diff = now.saturating_sub(secs);
    if diff < 60 {
        "just now".to_string()
    } else if diff < 3600 {
        format!("{}m ago", diff / 60)
    } else if diff < 86400 {
        format!("{}h ago", diff / 3600)
    } else if diff < 604800 {
        format!("{}d ago", diff / 86400)
    } else {
        match chrono::DateTime::from_timestamp(secs as i64, 0) {
            Some(dt) => dt.format("%b %d").to_string(),
            None => "unknown".to_string(),
        }
    }
}

pub fn verify_syntax(path: &str, workdir: &PathBuf) -> Result<(), String> {
    if path.ends_with(".rs") {
        let mut curr = workdir.clone();
        let mut found_cargo = false;
        for _ in 0..5 {
            if curr.join("Cargo.toml").exists() {
                found_cargo = true;
                break;
            }
            if let Some(parent) = curr.parent() {
                curr = parent.to_path_buf();
            } else {
                break;
            }
        }

        if found_cargo {
            let output = std::process::Command::new("cargo")
                .arg("check")
                .arg("--message-format=short")
                .current_dir(&curr)
                .output();

            match output {
                Ok(out) if !out.status.success() => {
                    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                    let combined = format!("{}\n{}", stdout, stderr);
                    return Err(format!("Cargo check failed after mutation:\n{}", combined.trim()));
                }
                Err(e) => return Err(format!("Failed to run cargo check: {}", e)),
                _ => {}
            }
        }
    }
    Ok(())
}
