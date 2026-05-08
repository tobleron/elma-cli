use std::path::{PathBuf};
use crate::tools::types::{ToolExecutionResult};
use crate::tools::helpers::{emit_tool_start, emit_tool_result};

pub async fn exec_git_inspect(
    av: &serde_json::Value,
    workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let mode = av["mode"].as_str().unwrap_or("status").to_string();
    let path = av["path"].as_str().unwrap_or("").to_string();

    let workdir = if path.is_empty() {
        workdir.clone()
    } else {
        workdir.join(&path)
    };

    let (args, description) = match mode.as_str() {
        "status" => (vec!["status", "--porcelain"], "git status"),
        "branch" => (vec!["branch", "-vv"], "git branch"),
        "changed_files" => (vec!["diff", "--name-only"], "git changed files"),
        "diffstat" => (vec!["diff", "--stat"], "git diff stat"),
        "recent_commits" => (vec!["log", "--oneline", "-10"], "git recent commits"),
        _ => {
            let error_msg = format!("Error: unknown mode '{}'", mode);
            return ToolExecutionResult::new_failed(call_id, "git_inspect", &error_msg);
        }
    };

    emit_tool_start(&mut tui, "git_inspect", description);

    let output = std::process::Command::new("git")
        .args(&args)
        .current_dir(&workdir)
        .output();

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let stderr = String::from_utf8_lossy(&o.stderr);
            let exit_code = o.status.code().unwrap_or(-1);
            let ok = o.status.success();

            let mut content = format!("Git {} (mode: {})\n", description, mode);
            if !stdout.is_empty() {
                content.push_str("--- stdout ---\n");
                content.push_str(&stdout);
            }
            if !stderr.is_empty() {
                content.push_str("\n--- stderr ---\n");
                content.push_str(&stderr);
            }
            content.push_str(&format!("\nExit code: {}", exit_code));

            emit_tool_result(&mut tui, "git_inspect", ok, &content);
            if ok {
                ToolExecutionResult::new_ok(call_id, "git_inspect", &content)
            } else {
                ToolExecutionResult::new_failed(call_id, "git_inspect", &content)
            }
        }
        Err(e) => {
            let msg = format!("Failed to execute git: {}", e);
            emit_tool_result(&mut tui, "git_inspect", false, &msg);
            ToolExecutionResult::new_failed(call_id, "git_inspect", &msg)
        }
    }
}
