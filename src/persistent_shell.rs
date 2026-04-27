//! @efficiency-role: service-orchestrator
//!
//! Persistent Guarded Shell (Task 288)
//!
//! Maintains a long-running shell process to solve profile noise issues
//! and provide state persistence (cd, export) across tool calls.

use crate::*;
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::io::{BufRead, BufReader, Read, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub(crate) struct PersistentShell {
    master: Box<dyn MasterPty + Send>,
    reader: BufReader<Box<dyn Read + Send>>,
    marker: String,
}

impl PersistentShell {
    pub(crate) fn new(workdir: &PathBuf) -> Result<Self> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 120,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| anyhow::anyhow!("Failed to open PTY: {}", e))?;

        let shell_owned: String;
        let shell = if cfg!(windows) {
            "powershell.exe"
        } else {
            shell_owned = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
            &shell_owned
        };

        let mut builder = CommandBuilder::new(shell);
        // Task 290: Do NOT use login shell (-l flag) to avoid profile noise.
        // Instead, inject a clean baseline environment.
        if !cfg!(windows) {
            // Inject clean environment variables
            let baseline_env = env_utils::get_baseline_environment();
            for (key, value) in baseline_env {
                builder.env(key, value);
            }
            // Suppress shell prompt (PS1/PS2) so it is not captured as output.
            // The prompt is written to the PTY and gets mixed into command output.
            builder.env("PS1", "");
            builder.env("PS2", "");
        }
        builder.cwd(workdir);

        let mut _child = pair
            .slave
            .spawn_command(builder)
            .map_err(|e| anyhow::anyhow!("Failed to spawn shell: {}", e))?;

        drop(pair.slave);

        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| anyhow::anyhow!("Failed to clone reader: {}", e))?;
        let marker = format!("__ELMA_SHELL_DONE_{}__", now_unix_s().unwrap_or(0));

        let mut shell = Self {
            master: pair.master,
            reader: BufReader::new(reader),
            marker,
        };

        shell.flush_initial_noise()?;

        Ok(shell)
    }

    fn flush_initial_noise(&mut self) -> Result<()> {
        // Disable echo to prevent commands from appearing in the output
        if !cfg!(windows) {
            let _ = self.master.write_all(b"stty -echo\r\n");
            let _ = self.master.flush();
        }

        let cmd = format!("echo '{}' $?\r\n", self.marker);
        self.master.write_all(cmd.as_bytes())?;
        self.master.flush()?;

        let mut line = String::new();
        let start = Instant::now();
        let timeout = Duration::from_secs(10);

        while start.elapsed() < timeout {
            line.clear();
            if self.reader.read_line(&mut line)? == 0 {
                break;
            }
            if line.trim().contains(&self.marker) {
                return Ok(());
            }
        }

        // Non-fatal warning if it fails, just move on
        eprintln!("Warning: Timed out flushing initial shell noise. Output may be slightly messy at first.");
        Ok(())
    }
    pub(crate) fn execute(&mut self, cmd: &str, timeout_secs: u64) -> Result<(i32, String)> {
        let full_cmd = if cfg!(windows) {
            format!("{}; \necho '{}' $LASTEXITCODE\r\n", cmd, self.marker)
        } else {
            format!("{}; \necho '{}' $?\r\n", cmd, self.marker)
        };

        self.master.write_all(full_cmd.as_bytes())?;
        self.master.flush()?;

        // Clone a fresh reader for this command. Using a single BufReader
        // across all commands caused stale buffered data to leak between
        // commands, producing wrong outputs and deadlocks.
        let reader = self
            .master
            .try_clone_reader()
            .map_err(|e| anyhow::anyhow!("Failed to clone PTY reader: {}", e))?;
        let mut buf_reader = BufReader::new(reader);
        let marker = self.marker.clone();

        let (tx, rx) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let mut output = String::new();
            let mut line = String::new();
            loop {
                line.clear();
                match buf_reader.read_line(&mut line) {
                    Ok(0) => {
                        let _ = tx.send(Err(anyhow::anyhow!(
                            "Shell EOF before finding marker"
                        )));
                        return;
                    }
                    Ok(_) => {
                        if line.contains(&marker)
                            && !line.contains(';')
                            && !line.contains("echo")
                        {
                            let exit_code = line
                                .split(&marker)
                                .nth(1)
                                .and_then(|s| s.trim().parse::<i32>().ok())
                                .unwrap_or(0);
                            let _ = tx.send(Ok((exit_code, output.trim().to_string())));
                            return;
                        }
                        output.push_str(&line);
                    }
                    Err(e) => {
                        let _ = tx.send(Err(anyhow::anyhow!("Shell read error: {}", e)));
                        return;
                    }
                }
            }
        });

        match rx.recv_timeout(Duration::from_secs(timeout_secs)) {
            Ok(result) => result,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Err(anyhow::anyhow!(
                "Shell command timed out after {}s",
                timeout_secs
            )),
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => Err(anyhow::anyhow!(
                "Shell reader thread disconnected unexpectedly"
            )),
        }
    }
}

pub(crate) type SharedShell = Arc<Mutex<PersistentShell>>;

pub(crate) fn get_shell(workdir: &PathBuf) -> Result<SharedShell> {
    static SHELL: OnceLock<SharedShell> = OnceLock::new();
    if let Some(s) = SHELL.get() {
        return Ok(s.clone());
    }
    let s = Arc::new(Mutex::new(PersistentShell::new(workdir)?));
    let _ = SHELL.set(s.clone());
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[tokio::test]
    async fn test_persistent_shell_state() -> Result<()> {
        let workdir = env::current_dir()?;
        let mut shell = PersistentShell::new(&workdir)?;

        // Test state persistence (cd)
        let (code1, _out1) = shell.execute("mkdir -p elma_test_dir && cd elma_test_dir", 5)?;
        assert_eq!(code1, 0);

        let (code2, out2) = shell.execute("pwd", 5)?;
        assert_eq!(code2, 0);
        println!("DEBUG PWD OUTPUT: [{}]", out2);
        assert!(out2.contains("elma_test_dir"));

        // Cleanup
        let _ = shell.execute("cd .. && rm -rf elma_test_dir", 5);
        Ok(())
    }
}
