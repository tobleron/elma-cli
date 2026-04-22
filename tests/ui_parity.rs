//! UI Parity Integration Tests
//!
//! Runs fixture-based tests to ensure terminal UI behavior
//! matches Claude Code parity spec.

use anyhow::{Context, Result};
use portable_pty::{native_pty_system, CommandBuilder, PtySize, PtySystem};
use serde::Deserialize;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Stdio;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio::time::sleep;

mod fixtures {
    use super::*;
    use std::path::PathBuf;

    #[derive(Debug, Clone, Deserialize)]
    pub struct Fixture {
        pub name: String,
        pub description: String,
        #[serde(default)]
        pub setup: Option<Setup>,
        pub steps: Vec<Step>,
        #[serde(default)]
        pub asserts: Option<Vec<Assert>>,
    }

    #[derive(Debug, Clone, Deserialize, Default)]
    pub struct Setup {
        #[serde(default)]
        pub env: Option<HashMap<String, String>>,
        #[serde(default)]
        pub args: Option<Vec<String>>,
        #[serde(default)]
        pub initial_files: Option<HashMap<String, String>>,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct Step {
        pub input: String,
        #[serde(default = "default_send_enter")]
        pub send_enter: bool,
        #[serde(default = "default_delay")]
        pub delay_ms: u64,
        #[serde(default)]
        pub wait_for: Option<String>,
        #[serde(default)]
        pub timeout_s: Option<u64>,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct Assert {
        pub pattern: String,
        #[serde(default)]
        pub not: bool,
        #[serde(default)]
        pub after_step: Option<usize>,
    }

    fn default_delay() -> u64 {
        0
    }

    fn default_send_enter() -> bool {
        true
    }

    impl Fixture {
        pub fn load(name: &str) -> Result<Self> {
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/ui_parity")
                .join(format!("{}.yaml", name));
            let contents = std::fs::read_to_string(&path)?;
            let fixture: Fixture = serde_yaml::from_str(&contents)?;
            Ok(fixture)
        }
    }
}

fn normalize_output(raw: &str) -> String {
    match strip_ansi_escapes::strip(raw) {
        Ok(s) => String::from_utf8_lossy(&s).to_string(),
        Err(_) => raw.to_string(),
    }
}

fn start_fake_server(response_delay_ms: u64) -> Result<String> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    let port = addr.port();

    thread::spawn(move || {
        for stream in listener.incoming() {
            if let Ok(mut stream) = stream {
                let mut req_buf = [0u8; 2048];
                let n = stream.read(&mut req_buf).unwrap_or(0);
                let req = String::from_utf8_lossy(&req_buf[..n]);
                let response = if req.contains("GET /v1/models") {
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"object\":\"list\",\"data\":[{\"id\":\"fake-model\",\"object\":\"model\",\"created\":0,\"owned_by\":\"test\"}]}"
                } else if req.contains("POST /v1/chat/completions") {
                    if response_delay_ms > 0 {
                        std::thread::sleep(Duration::from_millis(response_delay_ms));
                    }
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"id\":\"chatcmpl-test\",\"object\":\"chat.completion\",\"created\":0,\"model\":\"fake-model\",\"choices\":[{\"index\":0,\"message\":{\"role\":\"assistant\",\"content\":\"ok\"},\"finish_reason\":\"stop\"}]}"
                } else {
                    "HTTP/1.1 404 Not Found\r\nContent-Type: application/json\r\n\r\n{\"error\":\"not found\"}"
                };
                let _ = stream.write_all(response.as_bytes());
            }
        }
    });

    Ok(format!("http://127.0.0.1:{}/v1", port))
}

async fn run_fixture(fixture: &fixtures::Fixture) -> Result<String> {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut elma_bin = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    elma_bin.push("target/debug/elma-cli");
    if !elma_bin.exists() {
        elma_bin = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        elma_bin.push("target/release/elma-cli");
    }

    if !elma_bin.exists() {
        anyhow::bail!(
            "elma binary not found at {:?}. Run `cargo build` first.",
            elma_bin
        );
    }

    let pty_system = native_pty_system();
    let pty_pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .context("failed to open PTY")?;

    let mut cmd = CommandBuilder::new(&elma_bin);
    cmd.cwd(&project_root);

    // Isolate fixture runtime paths to avoid host config permission issues.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .context("system clock before unix epoch")?
        .as_nanos();
    let sandbox =
        std::env::temp_dir().join(format!("elma-ui-parity-{}-{}", std::process::id(), now));
    let config_root = sandbox.join("config");
    let sessions_root = sandbox.join("sessions");
    let home_root = sandbox.join("home");
    std::fs::create_dir_all(&config_root)?;
    std::fs::create_dir_all(&sessions_root)?;
    std::fs::create_dir_all(&home_root)?;
    cmd.arg("--config-root");
    cmd.arg(config_root.to_string_lossy().to_string());
    cmd.arg("--sessions-root");
    cmd.arg(sessions_root.to_string_lossy().to_string());
    cmd.env("HOME", home_root.to_string_lossy().to_string());

    if let Some(args) = fixture.setup.as_ref().and_then(|s| s.args.as_ref()) {
        for arg in args {
            cmd.arg(arg);
        }
    }

    let mut child = pty_pair
        .slave
        .spawn_command(cmd)
        .context("failed to spawn elma in PTY")?;

    let reader = pty_pair
        .master
        .try_clone_reader()
        .context("failed to clone reader")?;
    let writer = pty_pair
        .master
        .try_clone_writer()
        .context("failed to clone writer")?;

    let reader = Arc::new(Mutex::new(reader));
    let writer = Arc::new(Mutex::new(writer));

    // Give elma time to start up
    sleep(Duration::from_millis(1000)).await;

    // Execute fixture steps
    for (i, step) in fixture.steps.iter().enumerate() {
        eprintln!("Executing step {}: {:?}", i, step.input);

        // Send input
        {
            let mut writer_guard = writer.lock().await;
            writer_guard.write_all(step.input.as_bytes())?;
            if step.send_enter {
                writer_guard.write_all(b"\n")?;
            }
            writer_guard.flush()?;
        }

        // Wait for specified delay
        if step.delay_ms > 0 {
            sleep(Duration::from_millis(step.delay_ms)).await;
        }

        // If wait_for pattern is specified, wait for it
        if let Some(wait_for) = &step.wait_for {
            let timeout = step.timeout_s.unwrap_or(10);
            let start = std::time::Instant::now();
            loop {
                if start.elapsed() > Duration::from_secs(timeout) {
                    anyhow::bail!("Timeout waiting for pattern: {}", wait_for);
                }

                // Read current output
                let mut buffer = [0u8; 1024];
                let mut reader = reader.lock().await;
                if let Ok(n) = reader.read(&mut buffer) {
                    if n > 0 {
                        let chunk = String::from_utf8_lossy(&buffer[..n]);
                        if chunk.contains(wait_for) {
                            break;
                        }
                    }
                }

                sleep(Duration::from_millis(100)).await;
            }
        }
    }

    // Give time for final output to settle, then terminate child so PTY drains.
    sleep(Duration::from_millis(500)).await;
    let _ = child.kill();
    sleep(Duration::from_millis(150)).await;

    // Read final output (bounded; avoid hanging on blocking PTY reads).
    let mut output = Vec::new();
    {
        let start = std::time::Instant::now();
        let mut reader = reader.lock().await;
        let mut buffer = [0u8; 8192];
        while start.elapsed() < Duration::from_millis(600) {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => output.extend_from_slice(&buffer[..n]),
                Err(_) => break,
            }
        }
    }

    let output_str = String::from_utf8_lossy(&output).to_string();
    let normalized = normalize_output(&output_str);

    if let Some(asserts) = &fixture.asserts {
        for a in asserts {
            let found = normalized.contains(&a.pattern);
            if a.not {
                if found {
                    anyhow::bail!(
                        "fixture assert failed: forbidden pattern {:?} found in output",
                        a.pattern
                    );
                }
            } else if !found {
                anyhow::bail!(
                    "fixture assert failed: pattern {:?} not found in output",
                    a.pattern
                );
            }
            let _ = a.after_step;
        }
    }

    Ok(normalized)
}

fn with_fake_url(mut fixture: fixtures::Fixture, fake_url: &str) -> fixtures::Fixture {
    if let Some(setup) = &mut fixture.setup {
        if let Some(args) = &mut setup.args {
            if let Some(url_idx) = args
                .iter()
                .position(|arg| arg == "http://localhost:11434/v1")
            {
                args[url_idx] = fake_url.to_string();
            }
        }
    }
    fixture
}

async fn run_named_fixture(name: &str) -> anyhow::Result<String> {
    let fixture = fixtures::Fixture::load(name)?;
    let response_delay_ms = fixture
        .setup
        .as_ref()
        .and_then(|s| s.env.as_ref())
        .and_then(|env| env.get("FAKE_RESPONSE_DELAY_MS"))
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);

    let fake_url = start_fake_server(response_delay_ms)?;
    let fixture = with_fake_url(fixture, &fake_url);
    run_fixture(&fixture).await
}

#[tokio::test]
async fn startup_fixture() -> anyhow::Result<()> {
    let output = run_named_fixture("startup").await?;
    assert!(!output.is_empty(), "startup fixture produced no output");
    // Basic check that the PTY harness is working
    assert!(
        output.contains("Elma") || output.contains(">") || output.contains("Error"),
        "Unexpected startup output"
    );
    Ok(())
}

#[tokio::test]
async fn permission_gate_fixture() -> anyhow::Result<()> {
    let output = run_named_fixture("permission-gate").await?;
    // Should not hang, and should either deny the command or show modal
    // The key is that it completes without hanging
    assert!(
        !output.is_empty(),
        "permission gate fixture produced no output"
    );
    Ok(())
}

#[tokio::test]
async fn shell_tool_success_fixture() -> anyhow::Result<()> {
    let output = run_named_fixture("shell-tool-success").await?;
    assert!(
        !output.is_empty(),
        "shell-tool-success fixture produced no output"
    );
    Ok(())
}

#[tokio::test]
async fn shell_tool_failure_fixture() -> anyhow::Result<()> {
    let output = run_named_fixture("shell-tool-failure").await?;
    assert!(
        !output.is_empty(),
        "shell-tool-failure fixture produced no output"
    );
    Ok(())
}

#[tokio::test]
async fn permission_prompt_fixture() -> anyhow::Result<()> {
    let output = run_named_fixture("permission-prompt").await?;
    assert!(
        !output.is_empty(),
        "permission-prompt fixture produced no output"
    );
    Ok(())
}

#[tokio::test]
async fn collapsed_tool_output_fixture() -> anyhow::Result<()> {
    let output = run_named_fixture("collapsed-tool-output").await?;
    assert!(
        !output.is_empty(),
        "collapsed-tool-output fixture produced no output"
    );
    Ok(())
}

#[tokio::test]
async fn slash_picker_fixture() -> anyhow::Result<()> {
    let output = run_named_fixture("slash-picker").await?;
    assert!(
        !output.is_empty(),
        "slash-picker fixture produced no output"
    );
    Ok(())
}

#[tokio::test]
async fn file_picker_fixture() -> anyhow::Result<()> {
    let output = run_named_fixture("file-picker").await?;
    assert!(!output.is_empty(), "file-picker fixture produced no output");
    Ok(())
}

#[tokio::test]
async fn bash_mode_fixture() -> anyhow::Result<()> {
    let output = run_named_fixture("bash-mode").await?;
    assert!(!output.is_empty(), "bash-mode fixture produced no output");
    Ok(())
}

#[tokio::test]
async fn double_escape_clear_fixture() -> anyhow::Result<()> {
    let output = run_named_fixture("double-escape-clear").await?;
    assert!(
        !output.is_empty(),
        "double-escape-clear fixture produced no output"
    );
    Ok(())
}

#[tokio::test]
async fn double_ctrl_c_exit_fixture() -> anyhow::Result<()> {
    let output = run_named_fixture("double-ctrl-c-exit").await?;
    assert!(
        !output.is_empty(),
        "double-ctrl-c-exit fixture produced no output"
    );
    Ok(())
}

#[tokio::test]
async fn todo_create_fixture() -> anyhow::Result<()> {
    let output = run_named_fixture("todo-create").await?;
    assert!(!output.is_empty(), "todo-create fixture produced no output");
    Ok(())
}

#[tokio::test]
async fn todo_progress_checkmark_fixture() -> anyhow::Result<()> {
    let output = run_named_fixture("todo-progress-checkmark").await?;
    assert!(
        !output.is_empty(),
        "todo-progress-checkmark fixture produced no output"
    );
    Ok(())
}

#[tokio::test]
async fn todo_toggle_fixture() -> anyhow::Result<()> {
    let output = run_named_fixture("todo-toggle").await?;
    assert!(!output.is_empty(), "todo-toggle fixture produced no output");
    Ok(())
}

#[tokio::test]
async fn manual_compact_fixture() -> anyhow::Result<()> {
    let output = run_named_fixture("manual-compact").await?;
    assert!(
        !output.is_empty(),
        "manual-compact fixture produced no output"
    );
    Ok(())
}

#[tokio::test]
async fn auto_compact_fixture() -> anyhow::Result<()> {
    let output = run_named_fixture("auto-compact").await?;
    assert!(
        !output.is_empty(),
        "auto-compact fixture produced no output"
    );
    Ok(())
}

#[tokio::test]
async fn status_line_fixture() -> anyhow::Result<()> {
    let output = run_named_fixture("status-line").await?;
    assert!(!output.is_empty(), "status-line fixture produced no output");
    Ok(())
}

#[tokio::test]
async fn notification_fixture() -> anyhow::Result<()> {
    let output = run_named_fixture("notification").await?;
    assert!(
        !output.is_empty(),
        "notification fixture produced no output"
    );
    Ok(())
}

#[tokio::test]
async fn clear_fixture() -> anyhow::Result<()> {
    let output = run_named_fixture("clear").await?;
    assert!(!output.is_empty(), "clear fixture produced no output");
    Ok(())
}

#[tokio::test]
async fn resume_picker_fixture() -> anyhow::Result<()> {
    let output = run_named_fixture("resume-picker").await?;
    assert!(
        !output.is_empty(),
        "resume-picker fixture produced no output"
    );
    Ok(())
}

#[tokio::test]
async fn prompt_history_fixture() -> anyhow::Result<()> {
    let output = run_named_fixture("prompt-history").await?;
    assert!(
        !output.is_empty(),
        "prompt-history fixture produced no output"
    );
    Ok(())
}

#[tokio::test]
async fn graceful_exit_fixture() -> anyhow::Result<()> {
    let output = run_named_fixture("graceful-exit").await?;
    assert!(
        !output.is_empty(),
        "graceful-exit fixture produced no output"
    );
    Ok(())
}

#[tokio::test]
async fn noninteractive_output_fixture() -> anyhow::Result<()> {
    let output = run_named_fixture("noninteractive-output").await?;
    assert!(
        !output.is_empty(),
        "noninteractive-output fixture produced no output"
    );
    Ok(())
}

#[tokio::test]
async fn busy_queue_fixture() -> anyhow::Result<()> {
    let output = run_named_fixture("busy-queue").await?;
    assert!(!output.is_empty(), "busy-queue fixture produced no output");
    assert!(
        output.contains("first long request"),
        "busy-queue missing first submitted prompt"
    );
    assert!(
        output.contains("second queued request"),
        "busy-queue missing second queued prompt"
    );
    Ok(())
}

#[tokio::test]
async fn stress_input_during_streaming_fixture() -> anyhow::Result<()> {
    let output = run_named_fixture("stress-input-during-streaming").await?;
    assert!(
        !output.is_empty(),
        "stress-input-during-streaming fixture produced no output"
    );
    // The typed text should be visible in the output (input was captured)
    assert!(
        output.contains("abc123test"),
        "stress-input-during-streaming: typed text was not captured in output"
    );
    Ok(())
}

#[tokio::test]
async fn input_during_tool_execution_fixture() -> anyhow::Result<()> {
    let output = run_named_fixture("input-during-tool-execution").await?;
    assert!(
        !output.is_empty(),
        "input-during-tool-execution fixture produced no output"
    );
    assert!(
        output.contains("test-input-456"),
        "input-during-tool-execution: typed text was not captured during tool execution"
    );
    Ok(())
}



#[cfg(test)]
mod snapshot_tests {
    use super::*;
    use insta::assert_snapshot;

    #[tokio::test]
    async fn startup_snapshot() -> Result<()> {
        let fixture = fixtures::Fixture::load("startup")?;
        let output = run_fixture(&fixture).await?;
        assert_snapshot!("startup", output);
        Ok(())
    }
}
