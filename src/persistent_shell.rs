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

/// Strip ANSI escape sequences and control characters from shell output.
/// This is a safety net — the primary defense is using /bin/sh with
/// suppressed PS1/PS2 and stty -echo.  See docs/proposals/ for the
/// vt100-based approach that was considered but not adopted.
fn sanitize_shell_output(raw: &str) -> String {
    let stripped = match strip_ansi_escapes::strip(raw.as_bytes()) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).to_string(),
        Err(_) => raw.to_string(), // Fallback: return raw if stripping fails
    };
    // Remove remaining control characters except newlines and tabs
    stripped
        .chars()
        .filter(|c| *c == '\n' || *c == '\t' || !c.is_control())
        .collect()
}

pub(crate) struct PersistentShell {
    master: Box<dyn MasterPty + Send>,
    reader: BufReader<Box<dyn Read + Send>>,
    marker: String,
    workdir: PathBuf,
    dead: bool, // Set true when timeout/EOF indicates shell may be dead
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

        // Use /bin/sh (not the user's $SHELL) to avoid interactive-shell
        // noise: zsh/fish emit zle sequences, RPROMPT, fancy prompts, etc.
        // that corrupt PTY reads.  /bin/sh is minimal and respects PS1.
        let shell = if cfg!(windows) {
            "powershell.exe"
        } else {
            "/bin/sh"
        };

        let mut builder = CommandBuilder::new(shell);
        // Task 290: Do NOT use login shell (-l flag) to avoid profile noise.
        // Instead, inject a clean baseline environment.
        if !cfg!(windows) {
            let baseline_env = env_utils::get_baseline_environment();
            for (key, value) in baseline_env {
                builder.env(key, value);
            }
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
            workdir: workdir.clone(),
            dead: false,
        };

        shell.flush_initial_noise()?;

        Ok(shell)
    }

    fn flush_initial_noise(&mut self) -> Result<()> {
        // First: suppress prompt and echo.  builder.env("PS1","") does NOT
        // work because zsh/bash read their startup files and override it.
        // We must send the export commands AFTER the shell has started.
        let suppress = "export PS1=''; export PS2=''; stty -echo\r\n";
        self.master.write_all(suppress.as_bytes())?;
        self.master.flush()?;

        // Give the shell a moment to process the export commands
        std::thread::sleep(Duration::from_millis(100));

        // Now send a marker echo to confirm the shell is ready
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
            // After suppressing PS1, the output should be clean:
            // just the marker line and possibly the exit code.
            // We accept any line containing the marker.
            if line.contains(&self.marker) {
                return Ok(());
            }
        }

        // Non-fatal warning if it fails, just move on
        eprintln!("Warning: Timed out flushing initial shell noise. Output may be slightly messy at first.");
        Ok(())
    }

    pub(crate) fn execute(&mut self, cmd: &str, timeout_secs: u64) -> Result<(i32, String)> {
        // If shell is marked dead from a previous timeout/EOF, recreate it
        if self.dead {
            self.recreate()?;
        }

        // IMPORTANT: the marker echo must be on its own line.
        //
        // Appending a trailing `;` breaks heredoc terminators (`EOF;`), which can leave the
        // shell stuck waiting for the terminator and cause follow-up commands to fail.
        let full_cmd = if cfg!(windows) {
            format!("{}\n echo '{}' $LASTEXITCODE\r\n", cmd, self.marker)
        } else {
            format!("{}\n echo '{}' $?\r\n", cmd, self.marker)
        };

        self.master.write_all(full_cmd.as_bytes())?;
        self.master.flush()?;

        // Move the reader into a worker thread so blocking read_line() does
        // not freeze the main thread, and so we get a real timeout.
        // We MUST use the SAME reader for every command; creating a new
        // cloned reader per command causes a race between BufReaders on the
        // same PTY FD and produces garbage/missing output.
        let reader = std::mem::replace(
            &mut self.reader,
            BufReader::new(Box::new(std::io::empty()) as Box<dyn Read + Send>),
        );
        let marker = self.marker.clone();
        let (tx, rx) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let mut output = String::new();
            let mut line = String::new();
            let mut buf_reader = reader;
            loop {
                line.clear();
                match buf_reader.read_line(&mut line) {
                    Ok(0) => {
                        let _ = tx.send(Err((
                            anyhow::anyhow!("Shell EOF before finding marker"),
                            buf_reader,
                        )));
                        return;
                    }
                    Ok(_) => {
                        // Treat the marker line as complete only when we can parse an exit code
                        // from the suffix. This avoids false positives when the PTY echoes the
                        // input command text (which also includes the marker string).
                        if line.contains(&marker) {
                            if let Some(exit_code) = line
                                .split(&marker)
                                .nth(1)
                                .and_then(|s| s.trim().parse::<i32>().ok())
                            {
                                let _ =
                                    tx.send(Ok((exit_code, output.trim().to_string(), buf_reader)));
                                return;
                            }
                        }
                        output.push_str(&line);
                    }
                    Err(e) => {
                        let _ = tx.send(Err((
                            anyhow::anyhow!("Shell read error: {}", e),
                            buf_reader,
                        )));
                        return;
                    }
                }
            }
        });

        match rx.recv_timeout(Duration::from_secs(timeout_secs)) {
            Ok(Ok((exit_code, output, buf_reader))) => {
                self.reader = buf_reader;
                Ok((exit_code, sanitize_shell_output(&output)))
            }
            Ok(Err((e, buf_reader))) => {
                self.reader = buf_reader;
                // EOF means the shell process died — mark for recreation
                if e.to_string().contains("EOF") {
                    self.dead = true;
                }
                Err(e)
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // Timeout: the reader thread is still running and holds the real reader.
                // We've already swapped in std::io::empty() as a placeholder.
                // The shell process may still be running but we can't recover the reader
                // without waiting for the thread to finish or killing the process.
                // Mark as dead so next execute() recreates the shell.
                self.dead = true;
                Err(anyhow::anyhow!(
                    "Shell command timed out after {}s — the command is likely scanning too many files or blocking. Try a narrower search (add -maxdepth, specific paths), use ripgrep (rg) instead of find, or break the task into smaller steps.",
                    timeout_secs
                ))
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => Err(anyhow::anyhow!(
                "Shell reader thread disconnected unexpectedly"
            )),
        }
    }

    /// Recreate the shell session after timeout or EOF killed the previous one.
    fn recreate(&mut self) -> Result<()> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 120,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| anyhow::anyhow!("Failed to open PTY: {}", e))?;

        let shell = if cfg!(windows) {
            "powershell.exe"
        } else {
            "/bin/sh"
        };

        let mut builder = CommandBuilder::new(shell);
        if !cfg!(windows) {
            let baseline_env = env_utils::get_baseline_environment();
            for (key, value) in baseline_env {
                builder.env(key, value);
            }
        }
        builder.cwd(&self.workdir);

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

        self.master = pair.master;
        self.reader = BufReader::new(reader);
        self.marker = marker;
        self.dead = false;

        self.flush_initial_noise()?;

        Ok(())
    }
}

pub(crate) type SharedShell = Arc<Mutex<PersistentShell>>;

pub(crate) fn get_shell(workdir: &PathBuf) -> Result<SharedShell> {
    static SHELL: OnceLock<SharedShell> = OnceLock::new();
    if let Some(s) = SHELL.get() {
        return Ok(s.clone());
    }
    let s = Arc::new(Mutex::new(PersistentShell::new(workdir)?));
    // Store workdir in the shell for recreation
    let _ = SHELL.set(s.clone());
    Ok(s.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[tokio::test]
    async fn test_persistent_shell_state() -> Result<()> {
        let workdir = env::current_dir()?;
        let mut shell = PersistentShell::new(&workdir)?;

        // Test basic command
        let (code1, out1) = shell.execute("echo hello_world", 5)?;
        assert_eq!(code1, 0);
        assert!(
            out1.contains("hello_world"),
            "Expected 'hello_world' in output, got: [{}]",
            out1
        );

        // Test state persistence (cd)
        let (code2, _out2) = shell.execute("mkdir -p elma_test_dir && cd elma_test_dir", 5)?;
        assert_eq!(code2, 0);

        let (code3, out3) = shell.execute("pwd", 5)?;
        assert_eq!(code3, 0);
        println!("DEBUG PWD OUTPUT: [{}]", out3);
        assert!(out3.contains("elma_test_dir"));

        // Cleanup
        let _ = shell.execute("cd .. && rm -rf elma_test_dir", 5);
        Ok(())
    }

    #[test]
    fn test_persistent_shell_date_and_pwd() -> Result<()> {
        let workdir = env::current_dir()?;
        let mut shell = PersistentShell::new(&workdir)?;

        let (code, out) = shell.execute("date && pwd", 5)?;
        assert_eq!(code, 0);
        println!("DEBUG date&&pwd OUTPUT: [{}]", out);

        // Output should contain a date (year) and the current directory
        assert!(
            out.contains("2026") || out.contains("2025"),
            "Expected date output, got: [{}]",
            out
        );
        assert!(
            out.contains("elma-cli"),
            "Expected pwd output, got: [{}]",
            out
        );

        Ok(())
    }

    #[test]
    fn test_persistent_shell_find_agents_md() -> Result<()> {
        let workdir = env::current_dir()?;
        let mut shell = PersistentShell::new(&workdir)?;

        // Reproduce the exact command from session s_1777312219
        let (code, out) =
            shell.execute(r#"find . -name "AGENTS.md" 2>/dev/null | head -n 5"#, 10)?;
        assert_eq!(code, 0);
        println!("DEBUG find AGENTS.md OUTPUT:\n[{}]", out);

        // The root AGENTS.md MUST be found
        assert!(
            out.contains("./AGENTS.md"),
            "Expected ./AGENTS.md in output, got: [{}]",
            out
        );

        // There should be multiple results (at least root + qwen-code)
        let line_count = out.lines().count();
        assert!(
            line_count >= 2,
            "Expected at least 2 AGENTS.md files, got {} lines: [{}]",
            line_count,
            out
        );

        Ok(())
    }

    #[test]
    fn test_shell_recovery_after_timeout() -> Result<()> {
        let workdir = env::current_dir()?;
        let mut shell = PersistentShell::new(&workdir)?;

        // First: issue a command that will timeout (sleep 30s with 2s timeout)
        let result = shell.execute("sleep 30", 2);
        assert!(result.is_err(), "Expected timeout error");
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("timed out"),
            "Expected timeout error, got: {}",
            err
        );

        // Second: immediately issue a fast command — should auto-recover
        let (code, out) = shell.execute("echo recovered", 5)?;
        assert_eq!(code, 0);
        assert!(
            out.contains("recovered"),
            "Expected 'recovered' in output after timeout, got: [{}]",
            out
        );

        // Third: verify shell is fully functional
        let (code2, out2) = shell.execute("echo still_alive", 5)?;
        assert_eq!(code2, 0);
        assert!(
            out2.contains("still_alive"),
            "Expected 'still_alive' in output, got: [{}]",
            out2
        );

        Ok(())
    }

    #[test]
    fn test_shell_recovery_after_eof() -> Result<()> {
        let workdir = env::current_dir()?;
        let mut shell = PersistentShell::new(&workdir)?;

        // Normal command first
        let (code, out) = shell.execute("echo before_death", 5)?;
        assert_eq!(code, 0);
        assert!(out.contains("before_death"));

        // Manually mark shell as dead (simulates EOF)
        shell.dead = true;

        // Next command should auto-recover
        let (code2, out2) = shell.execute("echo after_recovery", 5)?;
        assert_eq!(code2, 0);
        assert!(
            out2.contains("after_recovery"),
            "Expected 'after_recovery' after simulated EOF, got: [{}]",
            out2
        );

        Ok(())
    }
}
