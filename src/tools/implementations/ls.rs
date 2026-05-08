use std::path::{Path, PathBuf};
use crate::tools::types::{ToolExecutionResult, ToolStatus};
use crate::tools::helpers::{emit_tool_start, emit_tool_result, format_time};
use crate::program_utils::resolve_tool_path;

pub fn exec_ls(
    av: &serde_json::Value,
    workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let raw_path = av["path"].as_str().unwrap_or("").to_string();
    let depth = av["depth"].as_i64().unwrap_or(2).clamp(1, 5) as usize;
    let ignore_patterns: Vec<String> = av["ignore"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let target = if raw_path.is_empty() {
        workdir.clone()
    } else {
        match resolve_tool_path(workdir, &raw_path) {
            Ok(p) => p,
            Err(e) => {
                let error_msg = format!("path error: {}", e);
                emit_tool_result(&mut tui, "ls", false, &error_msg);
                return ToolExecutionResult::new_failed(call_id, "ls", &error_msg);
            }
        }
    };

    emit_tool_start(&mut tui, "ls", &raw_path);

    let md = match std::fs::symlink_metadata(&target) {
        Ok(m) => m,
        Err(e) => {
            let error_msg = format!("Error accessing {}: {}", target.display(), e);
            emit_tool_result(&mut tui, "ls", false, &error_msg);
            return ToolExecutionResult::new_failed(call_id, "ls", &error_msg);
        }
    };

    if md.is_file() {
        let modified = md
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| format_time(d.as_secs()))
            .unwrap_or_default();
        let content = format!(
            "File: {}  ({} B, modified {})",
            target.file_name().unwrap_or_default().to_string_lossy(),
            md.len(),
            modified
        );
        emit_tool_result(&mut tui, "ls", true, &content);
        return ToolExecutionResult::new_ok(call_id, "ls", &content);
    }

    if !md.is_dir() {
        let error_msg = format!("Not a directory or file: {}", target.display());
        emit_tool_result(&mut tui, "ls", false, &error_msg);
        return ToolExecutionResult::new_failed(call_id, "ls", &error_msg);
    }

    let mut entries: Vec<LsEntry> = Vec::new();
    let total_count = collect_entries(&target, &target, depth, &ignore_patterns, &mut entries);

    let max_entries = 1000;
    let truncated = entries.len() > max_entries;
    if truncated {
        entries.truncate(max_entries);
    }

    let mut lines = Vec::new();
    let display_name = if raw_path.is_empty() {
        ".".to_string()
    } else {
        raw_path.clone()
    };
    lines.push(format!("{}/  ({} item(s))", display_name, total_count));

    for entry in &entries {
        let indent = "    ".repeat(entry.depth);
        let modified = format_time(entry.modified_secs);
        let size_str = if entry.is_dir {
            String::new()
        } else {
            format!("  ({} B, {})", entry.size, modified)
        };
        let suffix = if entry.is_dir { "/" } else { "" };
        lines.push(format!("{}{}{}{}", indent, entry.name, suffix, size_str));
    }

    if truncated {
        lines.push(format!(
            "... and {} more entries",
            total_count.saturating_sub(max_entries)
        ));
    }

    let content = lines.join("\n");
    emit_tool_result(&mut tui, "ls", true, &content);
    ToolExecutionResult::new_ok(call_id, "ls", &content)
}

struct LsEntry {
    name: String,
    depth: usize,
    is_dir: bool,
    size: u64,
    modified_secs: u64,
}

fn collect_entries(
    root: &Path,
    dir: &Path,
    max_depth: usize,
    ignore_patterns: &[String],
    entries: &mut Vec<LsEntry>,
) -> usize {
    let current_depth = if dir == root {
        0
    } else {
        dir.strip_prefix(root)
            .map(|p| p.components().count())
            .unwrap_or(0)
    };

    if current_depth > max_depth {
        return 0;
    }

    let read_dir = match std::fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return 0,
    };

    let mut local: Vec<LsEntry> = Vec::new();
    let mut total: usize = 0;

    for entry in read_dir.filter_map(|e| e.ok()) {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        if is_ignored(&name, ignore_patterns) {
            continue;
        }
        total += 1;

        let ft = match entry.file_type() {
            Ok(t) => t,
            Err(_) => continue,
        };

        let is_dir = ft.is_dir();
        let md = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        let modified_secs = md
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        local.push(LsEntry {
            name,
            depth: current_depth,
            is_dir,
            size: md.len(),
            modified_secs,
        });

        if is_dir && current_depth < max_depth {
            total += collect_entries(root, &entry.path(), max_depth, ignore_patterns, entries);
        }
    }

    local.sort_by(|a, b| {
        if a.is_dir != b.is_dir {
            b.is_dir.cmp(&a.is_dir)
        } else {
            a.name.cmp(&b.name)
        }
    });

    entries.extend(local);
    total
}

fn is_ignored(name: &str, patterns: &[String]) -> bool {
    for pattern in patterns {
        if let Ok(true) = glob::Pattern::new(pattern).map(|p| p.matches(name)) {
            return true;
        }
    }
    false
}
