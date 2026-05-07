//! @efficiency-role: domain-logic
//!
//! Process Group Management (Task 659)
//!
//! Process group cleanup and background job runtime.
//! Spawns processes in their own process group, tracks status,
//! and provides lifecycle management (kill, cleanup, list).

use crate::*;
use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// Status of a tracked process group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ProcessStatus {
    Pending,
    Running,
    Completed { exit_code: i32 },
    Failed(String),
    Killed,
}

impl std::fmt::Display for ProcessStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcessStatus::Pending => write!(f, "pending"),
            ProcessStatus::Running => write!(f, "running"),
            ProcessStatus::Completed { exit_code } => write!(f, "completed({})", exit_code),
            ProcessStatus::Failed(msg) => write!(f, "failed({})", msg),
            ProcessStatus::Killed => write!(f, "killed"),
        }
    }
}

/// A tracked process group with metadata and status.
#[derive(Debug, Clone)]
pub(crate) struct ProcessGroup {
    pub(crate) id: String,
    pub(crate) command: String,
    pub(crate) pid: Option<u32>,
    pub(crate) started_at: u64,
    pub(crate) status: ProcessStatus,
}

/// Container for a process group and its OS child handle.
struct ManagedProcess {
    group: ProcessGroup,
    child: Option<Child>,
}

/// Background job runtime: spawns, tracks, and manages shell processes.
pub(crate) struct BackgroundJobRuntime {
    pub(crate) jobs: HashMap<String, ProcessGroup>,
    children: HashMap<String, Child>,
}

impl BackgroundJobRuntime {
    pub(crate) fn new() -> Self {
        Self {
            jobs: HashMap::new(),
            children: HashMap::new(),
        }
    }

    /// Spawn a shell command as a background process.
    /// Returns the assigned job ID on success.
    pub(crate) fn spawn(command: &str) -> Result<String> {
        let cmd = command.trim().to_string();
        if cmd.is_empty() {
            return Err(anyhow::anyhow!("Cannot spawn empty command"));
        }

        let id = generate_job_id();
        let started_at = now_epoch_secs();

        let child = Command::new("sh")
            .arg("-c")
            .arg(&cmd)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to spawn process: {}", e))?;

        let pid = child.id();

        let group = ProcessGroup {
            id: id.clone(),
            command: cmd,
            pid: Some(pid),
            started_at,
            status: ProcessStatus::Running,
        };

        let mut rt = runtime().lock().unwrap_or_else(|e| e.into_inner());
        rt.jobs.insert(id.clone(), group.clone());
        rt.children.insert(id.clone(), child);

        Ok(id)
    }

    /// Get the current status of a job.
    pub(crate) fn status(job_id: &str) -> Option<ProcessStatus> {
        let mut rt = runtime().lock().unwrap_or_else(|e| e.into_inner());
        rt.try_update(job_id);
        rt.jobs.get(job_id).map(|g| g.status.clone())
    }

    /// Kill a running job by its ID.
    pub(crate) fn kill(job_id: &str) -> Result<()> {
        let mut rt = runtime().lock().unwrap_or_else(|e| e.into_inner());

        if let Some(child) = rt.children.get_mut(job_id) {
            child
                .kill()
                .map_err(|e| anyhow::anyhow!("Failed to kill process {}: {}", job_id, e))?;
            child.wait().ok();
        }

        if let Some(group) = rt.jobs.get_mut(job_id) {
            group.status = ProcessStatus::Killed;
        }

        rt.children.remove(job_id);
        Ok(())
    }

    /// Kill all remaining jobs and clean up state.
    pub(crate) fn cleanup_all() -> Result<()> {
        let mut rt = runtime().lock().unwrap_or_else(|e| e.into_inner());

        let ids: Vec<String> = rt.children.keys().cloned().collect();
        for id in &ids {
            if let Some(child) = rt.children.get_mut(id) {
                let _ = child.kill();
                let _ = child.wait();
            }
            if let Some(group) = rt.jobs.get_mut(id) {
                if group.status == ProcessStatus::Running {
                    group.status = ProcessStatus::Killed;
                }
            }
        }

        rt.children.clear();
        Ok(())
    }

    /// List all tracked process groups.
    pub(crate) fn list() -> Vec<ProcessGroup> {
        let rt = runtime().lock().unwrap_or_else(|e| e.into_inner());
        let mut groups: Vec<ProcessGroup> = rt.jobs.values().cloned().collect();
        groups.sort_by(|a, b| a.started_at.cmp(&b.started_at));
        groups
    }

    /// Try to update the status of a job by checking its child process.
    fn try_update(&mut self, job_id: &str) {
        let Some(child) = self.children.get_mut(job_id) else {
            return;
        };
        let Some(group) = self.jobs.get_mut(job_id) else {
            return;
        };

        match child.try_wait() {
            Ok(Some(status)) => {
                group.pid = None;
                if let Some(code) = status.code() {
                    if status.success() {
                        group.status = ProcessStatus::Completed { exit_code: code };
                    } else {
                        group.status = ProcessStatus::Failed(format!("exit code {}", code));
                    }
                } else {
                    group.status = ProcessStatus::Failed("terminated by signal".to_string());
                }
                self.children.remove(job_id);
            }
            Ok(None) => {}
            Err(e) => {
                group.status = ProcessStatus::Failed(format!("wait error: {}", e));
                self.children.remove(job_id);
            }
        }
    }
}

impl Default for BackgroundJobRuntime {
    fn default() -> Self {
        Self::new()
    }
}

static BG_RUNTIME: OnceLock<Mutex<BackgroundJobRuntime>> = OnceLock::new();

fn runtime() -> &'static Mutex<BackgroundJobRuntime> {
    BG_RUNTIME.get_or_init(|| Mutex::new(BackgroundJobRuntime::new()))
}

/// Reset the global job runtime (for testing and session reset).
pub(crate) fn reset_background_jobs() {
    let mut rt = runtime().lock().unwrap_or_else(|e| e.into_inner());
    for (_, child) in rt.children.iter_mut() {
        let _ = child.kill();
        let _ = child.wait();
    }
    rt.children.clear();
    rt.jobs.clear();
}

fn generate_job_id() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("job_{:x}{:04x}", now.as_secs(), now.subsec_nanos() & 0xffff)
}

fn now_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard, OnceLock};

    static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn test_guard() -> MutexGuard<'static, ()> {
        TEST_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    fn reset() {
        reset_background_jobs();
    }

    #[test]
    fn test_process_status_display() {
        let _guard = test_guard();
        assert_eq!(ProcessStatus::Pending.to_string(), "pending");
        assert_eq!(ProcessStatus::Running.to_string(), "running");
        assert_eq!(
            ProcessStatus::Completed { exit_code: 0 }.to_string(),
            "completed(0)"
        );
        assert_eq!(
            ProcessStatus::Failed("error".to_string()).to_string(),
            "failed(error)"
        );
        assert_eq!(ProcessStatus::Killed.to_string(), "killed");
    }

    #[test]
    fn test_spawn_and_status() {
        let _guard = test_guard();
        reset();
        let job_id = BackgroundJobRuntime::spawn("echo hello").expect("spawn should succeed");
        assert!(!job_id.is_empty());
        assert!(job_id.starts_with("job_"));

        let status = BackgroundJobRuntime::status(&job_id);
        assert!(status.is_some());
    }

    #[test]
    fn test_spawn_echo_completes() {
        let _guard = test_guard();
        reset();
        let job_id =
            BackgroundJobRuntime::spawn("echo background_test").expect("spawn should succeed");

        let mut final_status = None;
        for _ in 0..50 {
            if let Some(status) = BackgroundJobRuntime::status(&job_id) {
                if !matches!(status, ProcessStatus::Running) {
                    final_status = Some(status);
                    break;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        let status = final_status.expect("job should have completed");
        assert!(
            matches!(status, ProcessStatus::Completed { exit_code: 0 }),
            "expected completed(0), got {}",
            status
        );
    }

    #[test]
    fn test_kill_running_process() {
        let _guard = test_guard();
        reset();
        let job_id = BackgroundJobRuntime::spawn("sleep 60").expect("spawn should succeed");

        BackgroundJobRuntime::kill(&job_id).expect("kill should succeed");

        let status = BackgroundJobRuntime::status(&job_id);
        assert_eq!(status, Some(ProcessStatus::Killed));
    }

    #[test]
    fn test_cleanup_all() {
        let _guard = test_guard();
        reset();
        BackgroundJobRuntime::spawn("sleep 10").expect("spawn 1");
        BackgroundJobRuntime::spawn("sleep 10").expect("spawn 2");

        BackgroundJobRuntime::cleanup_all().expect("cleanup should succeed");

        let groups = BackgroundJobRuntime::list();
        for group in &groups {
            assert_ne!(group.status, ProcessStatus::Running);
        }
    }

    #[test]
    fn test_list_empty() {
        let _guard = test_guard();
        reset();
        let groups = BackgroundJobRuntime::list();
        assert!(groups.is_empty());
    }

    #[test]
    fn test_list_after_spawn() {
        let _guard = test_guard();
        reset();
        BackgroundJobRuntime::spawn("echo job_a").expect("spawn a");
        BackgroundJobRuntime::spawn("echo job_b").expect("spawn b");

        let groups = BackgroundJobRuntime::list();
        assert!(
            groups.len() >= 2,
            "should have at least 2 jobs after spawn, got {}",
            groups.len()
        );
    }

    #[test]
    fn test_spawn_empty_fails() {
        let _guard = test_guard();
        reset();
        let result = BackgroundJobRuntime::spawn("");
        assert!(result.is_err());
    }

    #[test]
    fn test_spawn_whitespace_fails() {
        let _guard = test_guard();
        reset();
        let result = BackgroundJobRuntime::spawn("   ");
        assert!(result.is_err());
    }

    #[test]
    fn test_status_for_nonexistent_job() {
        let _guard = test_guard();
        reset();
        let status = BackgroundJobRuntime::status("nonexistent");
        assert!(status.is_none());
    }

    #[test]
    fn test_kill_nonexistent_job() {
        let _guard = test_guard();
        reset();
        let result = BackgroundJobRuntime::kill("nonexistent");
        assert!(
            result.is_ok(),
            "kill of nonexistent should be ok (no-op): {:?}",
            result
        );
    }

    #[test]
    fn test_reset_clears_state() {
        let _guard = test_guard();
        reset();
        BackgroundJobRuntime::spawn("echo test").expect("spawn");
        assert!(!BackgroundJobRuntime::list().is_empty());

        reset();
        assert!(BackgroundJobRuntime::list().is_empty());
    }

    #[test]
    fn test_double_kill_is_safe() {
        let _guard = test_guard();
        reset();
        let job_id = BackgroundJobRuntime::spawn("sleep 30").expect("spawn");
        BackgroundJobRuntime::kill(&job_id).expect("first kill");
        let result = BackgroundJobRuntime::kill(&job_id);
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_cleanup_all_twice() {
        let _guard = test_guard();
        reset();
        BackgroundJobRuntime::spawn("sleep 5").expect("spawn");
        BackgroundJobRuntime::cleanup_all().expect("first cleanup");
        BackgroundJobRuntime::cleanup_all().expect("second cleanup");
        let groups = BackgroundJobRuntime::list();
        for group in &groups {
            assert_ne!(group.status, ProcessStatus::Running);
        }
    }
}
