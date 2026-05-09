//! @efficiency-role: util-pure
//!
//! Shell Timeout (Task 784)
//! Provides wall-clock timeout guards for shell commands in both sync and async contexts.

use crate::*;
use std::process::{Command, ExitStatus, Stdio, Output};
use std::time::{Duration, Instant};
use std::thread;

pub(crate) struct ShellTimeout;

impl ShellTimeout {
    /// Run a command synchronously with a hard wall-clock timeout.
    /// This is safe to call from sync contexts.
    pub(crate) fn run_sync(
        mut command: Command,
        timeout: Duration,
    ) -> Result<Output> {
        let mut child = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to spawn: {}", e))?;
            
        let deadline = Instant::now() + timeout;
        
        loop {
            match child.try_wait() {
                Ok(Some(_)) => {
                    // Command finished, but we need to collect output
                    return child.wait_with_output().map_err(|e| anyhow::anyhow!("Failed to wait for output: {}", e));
                }
                Ok(None) => {
                    if Instant::now() >= deadline {
                        let _ = child.kill();
                        return Err(anyhow::anyhow!("Command timed out after {:?}", timeout));
                    }
                    thread::sleep(Duration::from_millis(20));
                }
                Err(e) => return Err(anyhow::anyhow!("Error waiting for child: {}", e)),
            }
        }
    }

    /// Run a command asynchronously with a hard wall-clock timeout.
    /// Uses tokio::process::Command and tokio::time::timeout.
    pub(crate) async fn run_async(
        mut command: tokio::process::Command,
        timeout: Duration,
    ) -> Result<Output> {
        let future = command.output();
        match tokio::time::timeout(timeout, future).await {
            Ok(result) => result.map_err(|e| anyhow::anyhow!("Command execution failed: {}", e)),
            Err(_) => Err(anyhow::anyhow!("Command timed out after {:?}", timeout)),
        }
    }
}
