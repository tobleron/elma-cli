use crate::*;
use std::path::{Path, PathBuf};
use crate::tools::ToolExecutionResult;
use crate::tool_loop::ToolLoopSummary;

pub(crate) fn extract_read_paths_from_args(args_json: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(args_json) else {
        return Vec::new();
    };
    let Some(obj) = value.as_object() else {
        return Vec::new();
    };

    let mut paths = Vec::new();
    for key in ["path", "filePath"] {
        if let Some(path) = obj
            .get(key)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            paths.push(path.to_string());
        }
    }
    if let Some(arr) = obj.get("paths").and_then(|v| v.as_array()) {
        for path in arr
            .iter()
            .filter_map(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            paths.push(path.to_string());
        }
    }

    paths.sort();
    paths.dedup();
    paths
}

pub(crate) fn read_call_requests_broad_scope(paths: &[String]) -> bool {
    paths.len() > 1
        || paths
            .iter()
            .any(|path| path.contains('*') || path.contains('?') || path.contains('['))
}

pub(crate) fn concrete_workspace_file_path(workdir: &Path, candidate: &str) -> Option<String> {
    let clean = candidate.trim();
    if clean.is_empty() || clean.ends_with('/') {
        return None;
    }
    let full = if std::path::Path::new(clean).is_absolute() {
        PathBuf::from(clean)
    } else {
        workdir.join(clean)
    };
    if !full.is_file() {
        return None;
    }
    Some(
        full.strip_prefix(workdir)
            .unwrap_or(&full)
            .display()
            .to_string(),
    )
}

pub(crate) fn extract_ls_scope_paths(args_json: &str, output: &str, workdir: &Path) -> Vec<String> {
    let base = serde_json::from_str::<serde_json::Value>(args_json)
        .ok()
        .and_then(|v| v.get("path").and_then(|p| p.as_str()).map(str::to_string))
        .unwrap_or_default();
    let base = base.trim().trim_end_matches('/').to_string();
    let mut paths = Vec::new();

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed.contains("item(s)")
            || trimmed.starts_with("...")
            || trimmed.starts_with("total ")
            || trimmed.ends_with('/')
        {
            continue;
        }
        let name = if let Some((name, _meta)) = trimmed.rsplit_once("  (") {
            name.trim().to_string()
        } else {
            trimmed
                .split_whitespace()
                .next()
                .unwrap_or(trimmed)
                .to_string()
        };
        if name.is_empty() || name.contains("truncated") || name.ends_with("…") {
            continue;
        }
        let candidate = if base.is_empty() || base == "." {
            name
        } else {
            format!("{}/{}", base, name)
        };
        if let Some(path) = concrete_workspace_file_path(workdir, &candidate) {
            paths.push(path);
        }
    }

    paths.sort();
    paths.dedup();
    paths
}

pub(crate) fn extract_line_scope_paths(output: &str, workdir: &Path) -> Vec<String> {
    let mut paths = Vec::new();
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with("error")
            || trimmed.starts_with("Tool result")
            || trimmed.starts_with('[')
        {
            continue;
        }
        if let Some(path) = concrete_workspace_file_path(workdir, trimmed) {
            paths.push(path);
        }
    }
    paths.sort();
    paths.dedup();
    paths
}

pub(crate) fn update_scope_coverage_from_tool(
    scope_coverage: &mut crate::scope_coverage::ScopeCoverageLedger,
    tool_name: &str,
    args_json: &str,
    result: &ToolExecutionResult,
    workdir: &Path,
    scope_tracking_active: bool,
) {
    match tool_name {
        "ls" if result.ok => {
            let paths = extract_ls_scope_paths(args_json, &result.content, workdir);
            scope_coverage.register_items(&paths, "file");
        }
        "glob" if result.ok => {
            let paths = extract_line_scope_paths(&result.content, workdir);
            scope_coverage.register_items(&paths, "file");
        }
        "shell" if result.ok && scope_tracking_active => {
            let paths = extract_line_scope_paths(&result.content, workdir);
            scope_coverage.register_items(&paths, "file");
        }
        "read" => {
            let paths = extract_read_paths_from_args(args_json)
                .into_iter()
                .filter_map(|path| concrete_workspace_file_path(workdir, &path))
                .collect::<Vec<_>>();
            if !paths.is_empty() {
                scope_coverage.register_items(&paths, "file");
                for path in paths {
                    if result.ok {
                        scope_coverage.mark_covered(&path);
                    } else {
                        scope_coverage.mark_failed(&path);
                    }
                }
            }
        }
        _ => {}
    }
}

pub(crate) fn scope_coverage_pair(
    scope_coverage: &crate::scope_coverage::ScopeCoverageLedger,
) -> Option<(usize, usize)> {
    let total = scope_coverage.total();
    if total == 0 {
        None
    } else {
        Some((
            scope_coverage.count_by_status(crate::scope_coverage::CoverageStatus::Covered),
            total,
        ))
    }
}

pub(crate) fn sync_loop_summary_coverage(
    summary: &mut ToolLoopSummary,
    scope_coverage: &crate::scope_coverage::ScopeCoverageLedger,
) {
    summary.coverage = scope_coverage_pair(scope_coverage);
}

pub(crate) fn scope_coverage_blocks_finalization(
    read_scope_required: bool,
    scope_coverage: &crate::scope_coverage::ScopeCoverageLedger,
) -> bool {
    read_scope_required
        && (scope_coverage.total() == 0 || scope_coverage.has_pending())
}

pub(crate) fn build_scope_coverage_nudge(
    scope_coverage: &crate::scope_coverage::ScopeCoverageLedger,
) -> String {
    let pending = scope_coverage
        .items
        .iter()
        .filter(|item| item.status == crate::scope_coverage::CoverageStatus::Pending)
        .take(12)
        .map(|item| format!("- `{}`", item.item))
        .collect::<Vec<_>>();
    let failed = scope_coverage.count_by_status(crate::scope_coverage::CoverageStatus::Failed);
    let pending_suffix = if pending.is_empty() {
        String::new()
    } else {
        format!("\n\nPending files:\n{}", pending.join("\n"))
    };
    format!(
        "Scope coverage is incomplete: {}. Failed entries: {}. Continue reading the remaining concrete paths before answering. Prefer batching with the read tool's `paths` array when possible.{}",
        scope_coverage.render_summary(),
        failed,
        pending_suffix,
    )
}
