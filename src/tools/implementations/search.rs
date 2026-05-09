use std::path::{PathBuf};
use crate::tools::types::{ToolExecutionResult};
use crate::tools::helpers::{emit_tool_start, emit_tool_result, emit_tool_progress};
use crate::program_utils::resolve_tool_path;

pub async fn exec_search(
    av: &serde_json::Value,
    workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let pattern = av["pattern"].as_str().unwrap_or("").to_string();
    let sp = av["path"].as_str().map(String::from);
    if pattern.is_empty() {
        let error_msg = "Error: empty search pattern".to_string();
        emit_tool_result(&mut tui, "search", false, &error_msg);
        return ToolExecutionResult::new_failed(call_id, "search", &error_msg);
    }

    if let Some(ref p) = sp {
        if let Err(e) = resolve_tool_path(workdir, p) {
            let error_msg = format!("path error: {}", e);
            emit_tool_result(&mut tui, "search", false, &error_msg);
            return ToolExecutionResult::new_failed(call_id, "search", &error_msg);
        }
    }

    let mut cmd = std::process::Command::new("rg");
    cmd.arg("-i")
        .arg("--line-number")
        .arg("--no-heading")
        .arg("--color=never")
        .arg("--max-filesize")
        .arg("1M")
        .arg("--max-count")
        .arg("200");

    // Task 454: Honor literal_text and include schema fields
    let literal_text = av["literal_text"].as_bool().unwrap_or(false);
    if literal_text {
        cmd.arg("-F"); // Fixed string (literal) search
    }
    cmd.arg(&pattern);

    if let Some(include) = av["include"].as_str() {
        if !include.is_empty() {
            cmd.arg("--glob").arg(include);
        }
    } else if should_apply_default_search_exclusions(workdir, sp.as_deref()) {
        for glob in default_search_exclusion_globs() {
            cmd.arg("--glob").arg(glob);
        }
    }

    if let Some(p) = &sp {
        let search_path = workdir.join(p);
        if search_path.exists() {
            cmd.arg(&search_path);
        }
    } else {
        cmd.arg(workdir);
    }

    emit_tool_start(&mut tui, "search", &format!("rg pattern={}", pattern));
    emit_tool_progress(&mut tui, "search", "running ripgrep");

    let mut t_cmd = tokio::process::Command::from(cmd);
    match crate::shell_timeout::ShellTimeout::run_async(
        t_cmd,
        std::time::Duration::from_secs(30),
    ).await {
        Ok(output) => {
            let exit_code = output.status.code().unwrap_or(0);
            let success = exit_code == 0 || exit_code == 1;
            let mut content = if exit_code == 0 {
                String::from_utf8_lossy(&output.stdout).to_string()
            } else if exit_code == 1 {
                format!("No matches found for: {}", pattern)
            } else {
                format!(
                    "Search failed (exit {}):\n{}",
                    exit_code,
                    String::from_utf8_lossy(&output.stderr)
                )
            };

            // Task 542: Add annotation if matches include _knowledge_base
            if content.contains("_knowledge_base/") {
                let kb_count = content
                    .lines()
                    .filter(|l| l.contains("_knowledge_base/"))
                    .count();
                let total_count = content.lines().count();
                content.push_str(&format!(
                    "\n\nℹ️ NOTE: {} of {} matches are in _knowledge_base/ (third-party reference code). Exclude these from risk analysis of Elma's own codebase.",
                    kb_count, total_count
                ));
            }

            emit_tool_result(&mut tui, "search", success, &content);
            if success {
                ToolExecutionResult::new_ok(call_id, "search", &content)
            } else {
                ToolExecutionResult::new_failed(call_id, "search", &content)
            }
        }
        Err(e) => {
            let error_msg = format!("Search error: {}", e);
            emit_tool_result(&mut tui, "search", false, &error_msg);
            ToolExecutionResult::new_failed(call_id, "search", &error_msg)
        }
    }
}


fn should_apply_default_search_exclusions(workdir: &PathBuf, requested_path: Option<&str>) -> bool {
    let Some(path) = requested_path else {
        return true;
    };
    if path.trim().is_empty() || path == "." {
        return true;
    }
    let resolved = workdir.join(path);
    !crate::workspace_policy::WorkspacePolicy::path_is_default_excluded(&resolved)
}

fn default_search_exclusion_globs() -> Vec<String> {
    crate::workspace_policy::DEFAULT_EXCLUDED_PATHS
        .iter()
        .map(|path| format!("!{}/**", path))
        .collect()
}
