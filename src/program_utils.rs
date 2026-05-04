//! @efficiency-role: util-pure
//!
//! Program Utilities - Helpers for command execution and text processing

use crate::*;
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, Instant};

const MAX_INLINE_CAPTURE_BYTES: u64 = 1024 * 1024;
const MAX_ARTIFACT_BYTES: u64 = 8 * 1024 * 1024;
const MAX_WALL_SECS: u64 = 20;

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

pub(crate) fn resolve_tool_path(workdir: &Path, raw_path: &str) -> Result<PathBuf> {
    use std::path::Component;

    let raw_path = raw_path.trim();
    if raw_path.is_empty() {
        anyhow::bail!("tool path is empty");
    }
    let p = Path::new(raw_path);

    // Accept absolute paths that are inside the workspace (convert to relative).
    // Reject absolute paths outside the workspace.
    if p.is_absolute() {
        if let Ok(stripped) = p.strip_prefix(workdir) {
            // Absolute path inside workspace → treat as relative
            let relative = stripped;
            if relative.components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            }) {
                anyhow::bail!("tool path must stay inside the workspace");
            }
            return Ok(workdir.join(relative));
        } else {
            anyhow::bail!("absolute path outside workspace is not allowed");
        }
    }

    // Relative path: reject traversal escapes
    if p.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        anyhow::bail!("tool path must stay inside the workspace");
    }
    Ok(workdir.join(p))
}

// Keep old name as a shim so existing callers still compile during migration
pub(crate) fn resolve_workspace_edit_path(workdir: &Path, raw_path: &str) -> Result<PathBuf> {
    resolve_tool_path(workdir, raw_path)
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
    let (target_path, capture_limit, artifact_path, artifact_kind, _temp_file) =
        if let Some((path, kind)) = artifact_target {
            (
                path.clone(),
                MAX_ARTIFACT_BYTES,
                Some(path.clone()),
                Some(kind.to_string()),
                None,
            )
        } else {
            let temp_file = tempfile::Builder::new()
                .prefix("elma_shell_")
                .suffix(".out")
                .tempfile()
                .with_context(|| "failed to create temp file for shell output")?;
            (
                temp_file.path().to_path_buf(),
                MAX_INLINE_CAPTURE_BYTES,
                None,
                None,
                Some(temp_file),
            )
        };

    if let Ok(captured) = run_shell_one_liner_via_pty(cmd, workdir, capture_limit, MAX_WALL_SECS) {
        std::fs::write(&target_path, &captured.inline_text)
            .with_context(|| format!("write {}", target_path.display()))?;
        return Ok(ShellExecutionResult {
            exit_code: captured.exit_code,
            inline_text: finalize_shell_preview(
                captured.inline_text,
                captured.bytes_written,
                capture_limit,
                captured.truncated,
                captured.timed_out,
                artifact_path.is_some(),
            ),
            bytes_written: captured.bytes_written,
            truncated: captured.truncated,
            timed_out: captured.timed_out,
            artifact_path,
            artifact_kind,
        });
    }

    run_shell_one_liner_redirected(
        cmd,
        workdir,
        &target_path,
        capture_limit,
        artifact_path,
        artifact_kind,
    )
}

fn run_shell_one_liner_redirected(
    cmd: &str,
    workdir: &PathBuf,
    target_path: &Path,
    capture_limit: u64,
    artifact_path: Option<PathBuf>,
    artifact_kind: Option<String>,
) -> Result<ShellExecutionResult> {
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(target_path)
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

    let bytes = std::fs::read(target_path).unwrap_or_default();
    let bytes_written = bytes.len() as u64;
    let sanitized_output = sanitize_pty_transcript(&bytes);
    std::fs::write(target_path, &sanitized_output)
        .with_context(|| format!("rewrite {}", target_path.display()))?;
    let preview_limit = if artifact_path.is_some() {
        MAX_INLINE_CAPTURE_BYTES.min(4 * 1024)
    } else {
        MAX_INLINE_CAPTURE_BYTES
    };
    let raw_preview = sanitized_output
        .chars()
        .take(preview_limit as usize)
        .collect::<String>();
    let truncated = bytes_written >= capture_limit.saturating_sub(256);
    let inline_text = finalize_shell_preview(
        raw_preview,
        bytes_written,
        capture_limit,
        truncated,
        timed_out,
        artifact_path.is_some(),
    );

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

pub(crate) async fn run_shell_persistent(
    cmd: &str,
    workdir: &PathBuf,
) -> Result<ShellExecutionResult> {
    let shell_mutex = crate::persistent_shell::get_shell(workdir)?;
    let mut shell = shell_mutex
        .lock()
        .map_err(|_| anyhow::anyhow!("Shell mutex poisoned"))?;

    let start = Instant::now();
    let (exit_code, output) = shell.execute(cmd, MAX_WALL_SECS)?;
    let duration = start.elapsed();

    let bytes_written = output.len() as u64;
    let inline_text = sanitize_pty_transcript(output.as_bytes());

    Ok(ShellExecutionResult {
        exit_code,
        inline_text,
        bytes_written,
        truncated: false, // Persistent shell handles long output via marker
        timed_out: duration.as_secs() >= MAX_WALL_SECS,
        artifact_path: None,
        artifact_kind: None,
    })
}

pub(crate) fn run_shell_persistent_sync(
    cmd: &str,
    workdir: &PathBuf,
) -> Result<ShellExecutionResult> {
    let shell_mutex = crate::persistent_shell::get_shell(workdir)?;
    let mut shell = shell_mutex
        .lock()
        .map_err(|_| anyhow::anyhow!("Shell mutex poisoned"))?;

    let start = Instant::now();
    let (exit_code, output) = shell.execute(cmd, MAX_WALL_SECS)?;
    let duration = start.elapsed();

    let bytes_written = output.len() as u64;
    let inline_text = sanitize_pty_transcript(output.as_bytes());

    Ok(ShellExecutionResult {
        exit_code,
        inline_text,
        bytes_written,
        truncated: false,
        timed_out: duration.as_secs() >= MAX_WALL_SECS,
        artifact_path: None,
        artifact_kind: None,
    })
}

struct PtyCapture {
    exit_code: i32,
    inline_text: String,
    bytes_written: u64,
    truncated: bool,
    timed_out: bool,
}

fn run_shell_one_liner_via_pty(
    cmd: &str,
    workdir: &PathBuf,
    capture_limit: u64,
    max_wall_secs: u64,
) -> Result<PtyCapture> {
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 120,
            pixel_width: 0,
            pixel_height: 0,
        })
        .with_context(|| "open portable PTY")?;

    let blocks = capture_limit.div_ceil(512);
    let shell_script = format!("ulimit -f {blocks}; {cmd}");
    let mut builder = CommandBuilder::new("sh");
    builder.arg("-lc");
    builder.arg(&shell_script);
    builder.cwd(workdir);
    builder.env("TERM", "xterm-256color");

    let mut child = pair
        .slave
        .spawn_command(builder)
        .with_context(|| format!("Failed to run shell in PTY: {cmd}"))?;
    drop(pair.slave);

    let mut reader = pair
        .master
        .try_clone_reader()
        .with_context(|| "clone PTY reader")?;
    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    let reader_thread = std::thread::spawn(move || {
        let mut buffer = [0u8; 4096];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    if tx.send(buffer[..n].to_vec()).is_err() {
                        break;
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
    });

    let deadline = Instant::now() + Duration::from_secs(max_wall_secs);
    let mut raw_bytes = Vec::new();
    let mut timed_out = false;
    let mut truncated = false;
    let status = loop {
        while let Ok(chunk) = rx.try_recv() {
            raw_bytes.extend_from_slice(&chunk);
        }

        if raw_bytes.len() as u64 >= capture_limit && !truncated {
            truncated = true;
            let _ = child.kill();
        }

        if let Some(status) = child.try_wait().with_context(|| "poll PTY shell child")? {
            break status;
        }
        if Instant::now() >= deadline {
            timed_out = true;
            let _ = child.kill();
            break child
                .wait()
                .with_context(|| "wait killed PTY shell child")?;
        }
        std::thread::sleep(Duration::from_millis(20));
    };

    let _ = reader_thread.join();
    while let Ok(chunk) = rx.try_recv() {
        raw_bytes.extend_from_slice(&chunk);
    }
    if raw_bytes.len() as u64 > capture_limit {
        raw_bytes.truncate(capture_limit as usize);
        truncated = true;
    }

    Ok(PtyCapture {
        exit_code: if timed_out {
            124
        } else if status.success() {
            0
        } else {
            1
        },
        inline_text: sanitize_pty_transcript(&raw_bytes),
        bytes_written: raw_bytes.len() as u64,
        truncated,
        timed_out,
    })
}

fn sanitize_pty_transcript(bytes: &[u8]) -> String {
    let mut output = String::new();
    let mut current_line = String::new();
    let text = String::from_utf8_lossy(bytes);
    let mut chars = text.chars().peekable();
    let mut in_csi = false;
    let mut in_osc = false;

    while let Some(ch) = chars.next() {
        if in_osc {
            match ch {
                '\u{7}' => in_osc = false,
                '\u{1b}' if chars.peek() == Some(&'\\') => {
                    let _ = chars.next();
                    in_osc = false;
                }
                _ => {}
            }
            continue;
        }

        if in_csi {
            if ('@'..='~').contains(&ch) {
                in_csi = false;
            }
            continue;
        }

        match ch {
            '\u{1b}' => match chars.peek() {
                Some('[') => {
                    let _ = chars.next();
                    in_csi = true;
                }
                Some(']') => {
                    let _ = chars.next();
                    in_osc = true;
                }
                _ => {}
            },
            '\r' => current_line.clear(),
            '\n' => {
                while current_line.ends_with(' ') {
                    current_line.pop();
                }
                output.push_str(&current_line);
                output.push('\n');
                current_line.clear();
            }
            ch if ch.is_control() && ch != '\t' => {}
            _ => {
                current_line.push(ch);
            }
        }
    }

    if !current_line.is_empty() {
        while current_line.ends_with(' ') {
            current_line.pop();
        }
        output.push_str(&current_line);
    }

    output.trim_start().to_string()
}

fn finalize_shell_preview(
    mut inline_text: String,
    bytes_written: u64,
    capture_limit: u64,
    timed_out_or_truncated: bool,
    timed_out: bool,
    is_artifact: bool,
) -> String {
    let truncated = timed_out_or_truncated || bytes_written >= capture_limit.saturating_sub(256);
    if timed_out {
        if !inline_text.is_empty() && !inline_text.ends_with('\n') {
            inline_text.push('\n');
        }
        inline_text.push_str("[subterminal output stopped by Elma after time limit]\n");
    } else if truncated {
        if !inline_text.is_empty() && !inline_text.ends_with('\n') {
            inline_text.push('\n');
        }
        if is_artifact {
            inline_text.push_str("[subterminal artifact output truncated by Elma safety cap]\n");
        } else {
            inline_text.push_str("[subterminal output truncated by Elma safety cap]\n");
        }
    }
    inline_text
}

fn hash_short(text: &str) -> u64 {
    let mut hash = 1469598103934665603u64;
    for byte in text.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(1099511628211u64);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_pty_transcript_strips_ansi_sequences() {
        let input = b"\x1b[31mred\x1b[0m\nplain\n";
        assert_eq!(sanitize_pty_transcript(input), "red\nplain\n");
    }

    #[test]
    fn test_sanitize_pty_transcript_honors_carriage_returns() {
        let input = b"progress 10%\rprogress 100%\n";
        assert_eq!(sanitize_pty_transcript(input), "progress 100%\n");
    }
}
