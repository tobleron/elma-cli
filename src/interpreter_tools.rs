//! @efficiency-role: domain-logic
//!
//! Interpreter Tools - Local Code Interpreter Wrappers
//!
//! Provides structured execution for Python, Node, and other interpreters
//! using tokio::process directly (not shell strings).

use crate::*;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

/// Check if an interpreter is available.
pub(crate) fn is_interpreter_available(interpreter: &str) -> bool {
    #[cfg(target_os = "windows")]
    let cmd = format!("{} --version", interpreter);
    #[cfg(not(target_os = "windows"))]
    let cmd = format!("which {}", interpreter);

    std::process::Command::new("sh")
        .arg("-c")
        .arg(&cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Interpreter configuration.
pub(crate) struct InterpreterConfig {
    pub(crate) name: &'static str,
    pub(crate) binary: &'static str,
    pub(crate) file_extension: &'static str,
    pub(crate) available: bool,
}

/// Get all supported interpreters with availability check.
pub(crate) fn get_interpreters() -> Vec<InterpreterConfig> {
    let mut interpreters = vec![
        InterpreterConfig {
            name: "python",
            binary: "python3",
            file_extension: "py",
            available: false,
        },
        InterpreterConfig {
            name: "node",
            binary: "node",
            file_extension: "js",
            available: false,
        },
        InterpreterConfig {
            name: "ruby",
            binary: "ruby",
            file_extension: "rb",
            available: false,
        },
    ];

    for interp in &mut interpreters {
        interp.available = is_interpreter_available(interp.binary);
        if !interp.available {
            interp.available = is_interpreter_available(interp.name);
            if interp.available {
                interp.binary = interp.name;
            }
        }
    }

    interpreters
}

/// Execute code with an interpreter.
pub(crate) async fn execute_code(
    interpreter: &str,
    code: &str,
    workdir: &PathBuf,
    timeout_seconds: u64,
    max_output_lines: usize,
) -> Result<(String, String, i32), String> {
    let interpreters = get_interpreters();
    let interp_config = interpreters
        .iter()
        .find(|i| i.name == interpreter)
        .ok_or_else(|| format!("Unknown interpreter: {}", interpreter))?;

    if !interp_config.available {
        return Err(format!(
            "Interpreter '{}' is not available. Please install {} and try again.",
            interpreter, interp_config.binary
        ));
    }

    // Write code to a temporary file
    let temp_file = workdir.join(format!("temp_{}.{}", uuid_simple(), interp_config.file_extension));
    std::fs::write(&temp_file, code)
        .map_err(|e| format!("Failed to write temp file: {}", e))?;

    let timeout = Duration::from_secs(timeout_seconds);

    let mut child = Command::new(interp_config.binary)
        .arg(&temp_file)
        .current_dir(workdir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn {}: {}", interp_config.binary, e))?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let stdout_handle = if let Some(stdout) = stdout {
        let reader = BufReader::new(stdout);
        Some(tokio::spawn(async move {
            let mut lines = reader.lines();
            let mut output = Vec::new();
            while let Ok(Some(line)) = lines.next_line().await {
                if output.len() >= max_output_lines {
                    output.remove(0);
                }
                output.push(line);
            }
            output
        }))
    } else {
        None
    };

    let stderr_handle = if let Some(stderr) = stderr {
        let reader = BufReader::new(stderr);
        Some(tokio::spawn(async move {
            let mut lines = reader.lines();
            let mut output = Vec::new();
            while let Ok(Some(line)) = lines.next_line().await {
                if output.len() >= max_output_lines {
                    output.remove(0);
                }
                output.push(line);
            }
            output
        }))
    } else {
        None
    };

    let result = tokio::select! {
        result = child.wait() => result,
        _ = tokio::time::sleep(timeout) => {
            let _ = child.kill().await;
            return Err(format!(
                "Execution timed out after {} seconds",
                timeout_seconds
            ));
        }
    };

    // Clean up temp file
    let _ = std::fs::remove_file(&temp_file);

    let stdout_str = if let Some(handle) = stdout_handle {
        let lines = handle.await.unwrap_or_default();
        lines.join("\n")
    } else {
        String::new()
    };

    let stderr_str = if let Some(handle) = stderr_handle {
        let lines = handle.await.unwrap_or_default();
        lines.join("\n")
    } else {
        String::new()
    };

    let exit_code = result
        .map(|s| s.code().unwrap_or(-1))
        .unwrap_or(-1);

    Ok((stdout_str, stderr_str, exit_code))
}

fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{:x}{:x}", duration.as_secs(), duration.subsec_nanos())
}
