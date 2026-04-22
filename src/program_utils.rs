//! @efficiency-role: util-pure
//!
//! Program Utilities - Helpers for command execution and text processing

use crate::*;

pub(crate) fn should_classify_artifacts(
    complexity: &ComplexityAssessment,
    formula: &FormulaSelection,
) -> bool {
    formula.primary.eq_ignore_ascii_case("inspect_decide_reply")
        || complexity
            .suggested_pattern
            .eq_ignore_ascii_case("inspect_decide_reply")
}

pub(crate) fn edit_operation_is_supported(op: &str) -> bool {
    matches!(op.trim(), "write_file" | "replace_text" | "append_text")
}

pub(crate) fn resolve_workspace_edit_path(workdir: &Path, raw_path: &str) -> Result<PathBuf> {
    use std::path::Component;

    let raw_path = raw_path.trim();
    if raw_path.is_empty() {
        anyhow::bail!("edit path is empty");
    }
    let relative = Path::new(raw_path);
    if relative.is_absolute() {
        anyhow::bail!("absolute edit paths are not allowed");
    }
    if relative.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        anyhow::bail!("edit path must stay inside the workspace");
    }
    Ok(workdir.join(relative))
}

pub(crate) fn preview_text(text: &str, max_lines: usize) -> String {
    text.lines()
        .map(str::trim_end)
        .filter(|line| !line.trim().is_empty())
        .take(max_lines)
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn resolve_command_placeholders(
    cmd: &str,
    artifacts: &HashMap<String, String>,
) -> Result<String> {
    let mut out = String::new();
    let mut rest = cmd;

    while let Some(start) = rest.find("{{") {
        let (prefix, after_start) = rest.split_at(start);
        out.push_str(prefix);
        let after_start = &after_start[2..];
        let Some(end) = after_start.find("}}") else {
            anyhow::bail!("unclosed command placeholder");
        };
        let expr = after_start[..end].trim();
        let remainder = &after_start[end + 2..];
        let (id, mode) = expr
            .split_once('|')
            .map(|(id, mode)| (id.trim(), mode.trim()))
            .unwrap_or((expr, "raw"));
        let raw = artifacts
            .get(id)
            .with_context(|| format!("missing workflow artifact for placeholder {id}"))?;
        let value = match mode {
            "" | "raw" => raw.clone(),
            "shell_words" => raw
                .lines()
                .map(|line| line.trim())
                .filter(|line| !line.is_empty())
                .map(|line| line.trim_start_matches("- ").trim())
                .map(shell_quote)
                .collect::<Vec<_>>()
                .join(" "),
            _ => anyhow::bail!("unsupported placeholder mode {mode}"),
        };
        out.push_str(&value);
        rest = remainder;
    }
    out.push_str(rest);
    Ok(out)
}

pub(crate) fn command_placeholder_refs(cmd: &str) -> Vec<String> {
    let mut refs = Vec::new();
    let mut rest = cmd;
    while let Some(start) = rest.find("{{") {
        let after_start = &rest[start + 2..];
        let Some(end) = after_start.find("}}") else {
            break;
        };
        let expr = after_start[..end].trim();
        let id = expr
            .split_once('|')
            .map(|(id, _)| id.trim())
            .unwrap_or(expr);
        if !id.is_empty() && !refs.iter().any(|seen| seen == id) {
            refs.push(id.to_string());
        }
        rest = &after_start[end + 2..];
    }
    refs
}

pub(crate) async fn run_shell_one_liner(
    cmd: &str,
    workdir: &PathBuf,
    artifact_target: Option<(&PathBuf, &str)>,
) -> Result<ShellExecutionResult> {
    let cmd_owned = cmd.to_string();
    let workdir = workdir.clone();
    let artifact_target = artifact_target.map(|(p, k)| (p.clone(), k.to_string()));
    tokio::task::spawn_blocking(move || {
        run_shell_one_liner_blocking(
            &cmd_owned,
            &workdir,
            artifact_target.as_ref().map(|(p, k)| (p, k.as_str())),
        )
    })
    .await
    .with_context(|| "spawn_blocking for shell")?
}

/// Synchronous version for call sites that are not async.
pub(crate) fn run_shell_one_liner_sync(
    cmd: &str,
    workdir: &PathBuf,
    artifact_target: Option<(&PathBuf, &str)>,
) -> Result<ShellExecutionResult> {
    run_shell_one_liner_blocking(cmd, workdir, artifact_target)
}

fn run_shell_one_liner_blocking(
    cmd: &str,
    workdir: &PathBuf,
    artifact_target: Option<(&PathBuf, &str)>,
) -> Result<ShellExecutionResult> {
    const MAX_INLINE_CAPTURE_BYTES: u64 = 1024 * 1024;
    const MAX_ARTIFACT_BYTES: u64 = 8 * 1024 * 1024;
    const MAX_WALL_SECS: u64 = 20;

    let (target_path, capture_limit, artifact_path, artifact_kind) =
        if let Some((path, kind)) = artifact_target {
            (
                path.clone(),
                MAX_ARTIFACT_BYTES,
                Some(path.clone()),
                Some(kind.to_string()),
            )
        } else {
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let out_path = std::env::temp_dir().join(format!(
                "elma_shell_{}_{}_{}.out",
                std::process::id(),
                stamp,
                hash_short(cmd)
            ));
            (out_path, MAX_INLINE_CAPTURE_BYTES, None, None)
        };

    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&target_path)
        .with_context(|| format!("open {}", target_path.display()))?;
    let file_err = file
        .try_clone()
        .with_context(|| format!("clone {}", target_path.display()))?;

    let blocks = capture_limit.div_ceil(512);
    let shell_script = format!("ulimit -f {blocks}; {cmd}");
    let mut child = std::process::Command::new("sh")
        .arg("-lc")
        .arg(&shell_script)
        .current_dir(workdir)
        .stdout(std::process::Stdio::from(file))
        .stderr(std::process::Stdio::from(file_err))
        .spawn()
        .with_context(|| format!("Failed to run shell: {cmd}"))?;

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(MAX_WALL_SECS);
    let mut timed_out = false;
    let status = loop {
        if let Some(status) = child.try_wait().context("poll shell child")? {
            break status;
        }
        if std::time::Instant::now() >= deadline {
            timed_out = true;
            let _ = child.kill();
            break child.wait().context("wait killed shell child")?;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    };

    let bytes = std::fs::read(&target_path).unwrap_or_default();
    let bytes_written = bytes.len() as u64;
    let preview_limit = if artifact_path.is_some() {
        MAX_INLINE_CAPTURE_BYTES.min(4 * 1024)
    } else {
        MAX_INLINE_CAPTURE_BYTES
    };
    let mut inline_text =
        String::from_utf8_lossy(&bytes[..bytes.len().min(preview_limit as usize)]).to_string();
    let truncated = bytes_written >= capture_limit.saturating_sub(256);
    if timed_out {
        if !inline_text.is_empty() && !inline_text.ends_with('\n') {
            inline_text.push('\n');
        }
        inline_text.push_str("[output stopped by Elma after time limit]\n");
    } else if truncated {
        if !inline_text.is_empty() && !inline_text.ends_with('\n') {
            inline_text.push('\n');
        }
        if artifact_path.is_some() {
            inline_text.push_str("[artifact output truncated by Elma safety cap]\n");
        } else {
            inline_text.push_str("[output truncated by Elma safety cap]\n");
        }
    }

    if artifact_path.is_none() {
        let _ = std::fs::remove_file(&target_path);
    }

    Ok(ShellExecutionResult {
        exit_code: if timed_out {
            124
        } else {
            status.code().unwrap_or(1)
        },
        inline_text,
        bytes_written,
        truncated,
        timed_out,
        artifact_path,
        artifact_kind,
    })
}

fn hash_short(text: &str) -> u64 {
    let mut hash = 1469598103934665603u64;
    for byte in text.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(1099511628211u64);
    }
    hash
}
