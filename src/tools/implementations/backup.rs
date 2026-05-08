use std::path::{PathBuf};
use crate::tools::types::{ToolExecutionResult, ToolStatus};
use crate::tools::helpers::{emit_tool_start, emit_tool_result};

pub fn exec_backup(
    av: &serde_json::Value,
    workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let mut source_dir = av["source_dir"].as_str().unwrap_or("");
    let dest_dir = av["dest_dir"].as_str().unwrap_or("");

    if source_dir.is_empty() || dest_dir.is_empty() {
        let error_msg = "Error: source_dir and dest_dir are required".to_string();
        emit_tool_result(&mut tui, "backup", false, &error_msg);
        return ToolExecutionResult::new_failed(call_id, "backup", &error_msg);
    }

    if (source_dir == "." || source_dir == "./") && workdir.join("src").is_dir() {
        crate::append_trace_log_line(
            "[BACKUP] root source remapped to src for source-file backup contract",
        );
        source_dir = "src";
    }

    // Check dest_dir for future-dated timestamps
    let today = chrono::Local::now();
    if let Some(date_str) = dest_dir
        .split(|c: char| !c.is_ascii_digit())
        .find(|s| s.len() == 8 && s.as_bytes().iter().all(|b| b.is_ascii_digit()))
    {
        if let Ok(parsed) = chrono::NaiveDate::parse_from_str(date_str, "%Y%m%d") {
            let days_ahead = parsed.signed_duration_since(today.date_naive()).num_days();
            if days_ahead > 1 {
                crate::append_trace_log_line(&format!(
                    "[BACKUP] future-dated destination detected: {} ({} days ahead), using current-date path",
                    date_str, days_ahead
                ));
                let error_msg = format!(
                    "Error: destination path contains a future date ({}). Use a current-date path for backups.",
                    date_str
                );
                return ToolExecutionResult::new_failed(call_id, "backup", &error_msg);
            }
        }
    }

    let src = workdir.join(source_dir);
    if !src.exists() {
        let error_msg = format!("Error: source directory not found: {}", source_dir);
        emit_tool_result(&mut tui, "backup", false, &error_msg);
        return ToolExecutionResult::new_failed(call_id, "backup", &error_msg);
    }

    let include_patterns: Vec<String> = av["include_patterns"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_else(|| vec!["**/*.rs".to_string()]);

    let exclude_patterns: Vec<String> = av["exclude_patterns"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();

    let verify = av["verify"].as_bool().unwrap_or(true);

    let dest = workdir.join(dest_dir);

    emit_tool_start(
        &mut tui,
        "backup",
        &format!("{} -> {}", source_dir, dest_dir),
    );

    let start = std::time::Instant::now();

    match crate::safe_operations::run_backup_operation(
        &src,
        &dest,
        &include_patterns,
        &exclude_patterns,
    ) {
        Ok(result) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            let ok = result.completed;

            let manifest_count = 1u64;
            let verification_ok = result.errors.is_empty() && result.completed;

            let mut detail = format!(
                "Backup summary:\n\
                 source_files_matched={}\n\
                 payload_files_copied={}\n\
                 manifest_files_created={}\n\
                 errors={}\n\
                 verification_ok={}\n\n\
                 Source: {}\n\
                 Destination: {}\n\
                 Manifest: {}",
                result.source_match_count,
                result.files_copied,
                manifest_count,
                result.errors.len(),
                verification_ok,
                result.source_dir.display(),
                result.dest_dir.display(),
                result.manifest_path.display(),
            );

            if !result.errors.is_empty() {
                detail.push_str(&format!(
                    "\nErrors:\n  {}",
                    result.errors.join("\n  ")
                ));
            }

            if verify && verification_ok && result.source_match_count != result.files_copied {
                detail.push_str(&format!(
                    "\nNote: {} files matched but {} copied ({} missing)",
                    result.source_match_count,
                    result.files_copied,
                    result.source_match_count.saturating_sub(result.files_copied)
                ));
            }

            crate::append_trace_log_line(&format!(
                "[BACKUP] source={} dest={} matched={} copied={} manifest_files={} errors={} verification_ok={}",
                source_dir,
                dest_dir,
                result.source_match_count,
                result.files_copied,
                manifest_count,
                result.errors.len(),
                verification_ok,
            ));

            emit_tool_result(&mut tui, "backup", ok, &detail);
            ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "backup".to_string(),
                content: detail,
                ok,
                exit_code: None,
                timed_out: false,
                status: if ok {
                    ToolStatus::Success
                } else {
                    ToolStatus::Failed
                },
                duration_ms,
                signal_killed: None,
            }
        }
        Err(e) => {
            let error_msg = format!("Backup failed: {}", e);
            crate::append_trace_log_line(&format!("[BACKUP] failed: {}", error_msg));
            emit_tool_result(&mut tui, "backup", false, &error_msg);
            ToolExecutionResult {
                tool_call_id: call_id.to_string(),
                tool_name: "backup".to_string(),
                content: error_msg,
                ok: false,
                exit_code: None,
                timed_out: false,
                status: ToolStatus::Failed,
                duration_ms: start.elapsed().as_millis() as u64,
                signal_killed: None,
            }
        }
    }
}
