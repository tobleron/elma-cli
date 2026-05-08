use std::path::{PathBuf};
use crate::tools::types::{ToolExecutionResult};
use crate::tools::helpers::{emit_tool_start, emit_tool_result};
use crate::program_utils::resolve_tool_path;

pub fn exec_glob(
    av: &serde_json::Value,
    workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let pattern = av["pattern"].as_str().unwrap_or("*").to_string();
    let search_path = av["path"].as_str().map(PathBuf::from);

    emit_tool_start(&mut tui, "glob", &pattern);

    let base = match search_path {
        Some(p) => match resolve_tool_path(workdir, p.to_str().unwrap_or("")) {
            Ok(p) => p,
            Err(e) => {
                let error_msg = format!("path error: {}", e);
                emit_tool_result(&mut tui, "glob", false, &error_msg);
                return ToolExecutionResult::new_failed(call_id, "glob", &error_msg);
            }
        },
        None => workdir.clone(),
    };

    // Note: glob patterns are relative to the current working directory of the process
    // unless they are absolute. We should probably CD to base or use absolute pattern.
    // The original code used glob::glob_with directly on the pattern.
    
    let walker = glob::glob_with(
        &pattern,
        glob::MatchOptions {
            case_sensitive: false,
            require_literal_separator: false,
            require_literal_leading_dot: false,
        },
    );

    let mut results = Vec::new();
    let mut count = 0;
    let max_results = 100;

    if let Ok(walker) = walker {
        for entry in walker.filter_map(|e| e.ok()) {
            if count >= max_results {
                break;
            }
            let relative = entry
                .strip_prefix(workdir)
                .unwrap_or(&entry)
                .display()
                .to_string();
            results.push(relative);
            count += 1;
        }
    }

    let output = if results.is_empty() {
        "No files found matching pattern".to_string()
    } else {
        results.join("\n")
    };

    let ok = !results.is_empty();
    emit_tool_result(&mut tui, "glob", ok, &output);
    
    if ok {
        ToolExecutionResult::new_ok(call_id, "glob", &output)
    } else {
        ToolExecutionResult::new_failed(call_id, "glob", &output)
    }
}
