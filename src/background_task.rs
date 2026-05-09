//! @efficiency-role: domain-logic
//!
//! Background Task Management (Task 268)
//!
//! Provides formal background task management with memory limits.
//! Tasks run in background while main agent continues working.

use crate::*;
use std::collections::HashMap;
use std::path::PathBuf;
use shlex;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{broadcast, Mutex, RwLock};

static TASK_MANAGER: OnceLock<Arc<TaskManager>> = OnceLock::new();

/// Initialize the global task manager.
pub(crate) fn init_task_manager(config: BackgroundTaskConfig) {
    let _ = TASK_MANAGER.set(Arc::new(TaskManager::new(config)));
}

/// Get the global task manager.
pub(crate) fn get_task_manager() -> Option<&'static Arc<TaskManager>> {
    TASK_MANAGER.get()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BackgroundTaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    OOMKilled,
}

impl std::fmt::Display for BackgroundTaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackgroundTaskStatus::Pending => write!(f, "pending"),
            BackgroundTaskStatus::Running => write!(f, "running"),
            BackgroundTaskStatus::Completed => write!(f, "completed"),
            BackgroundTaskStatus::Failed => write!(f, "failed"),
            BackgroundTaskStatus::Cancelled => write!(f, "cancelled"),
            BackgroundTaskStatus::OOMKilled => write!(f, "oom_killed"),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct BackgroundTask {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) command: String,
    pub(crate) workdir: PathBuf,
    pub(crate) memory_limit_mb: u64,
    pub(crate) timeout_seconds: u64,
    pub(crate) status: BackgroundTaskStatus,
    pub(crate) exit_code: Option<i32>,
    pub(crate) started_at: Option<Instant>,
    pub(crate) memory_usage_mb: u64,
    pub(crate) stdout_buffer: Vec<String>,
    pub(crate) stderr_buffer: Vec<String>,
    pub(crate) max_output_lines: usize,
}

impl BackgroundTask {
    pub(crate) fn new(
        id: String,
        name: String,
        command: String,
        workdir: PathBuf,
        memory_limit_mb: u64,
        timeout_seconds: u64,
    ) -> Self {
        Self {
            id,
            name,
            command,
            workdir,
            memory_limit_mb,
            timeout_seconds,
            status: BackgroundTaskStatus::Pending,
            exit_code: None,
            started_at: None,
            memory_usage_mb: 0,
            stdout_buffer: Vec::new(),
            stderr_buffer: Vec::new(),
            max_output_lines: 1000,
        }
    }

    pub(crate) fn runtime_seconds(&self) -> Option<u64> {
        self.started_at.map(|started| {
            let elapsed = started.elapsed();
            elapsed.as_secs()
        })
    }

    pub(crate) fn is_memory_exceeded(&self) -> bool {
        self.memory_usage_mb > self.memory_limit_mb
    }
}

pub(crate) struct BackgroundTaskConfig {
    pub(crate) max_concurrent: usize,
    pub(crate) default_memory_limit_mb: u64,
    pub(crate) default_timeout_seconds: u64,
}

impl Default for BackgroundTaskConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 3,
            default_memory_limit_mb: 2048,
            default_timeout_seconds: 300,
        }
    }
}

pub(crate) struct TaskManager {
    tasks: RwLock<HashMap<String, Arc<Mutex<BackgroundTask>>>>,
    config: BackgroundTaskConfig,
    cancel_signals: broadcast::Sender<String>,
    system_memory_kb: u64,
}

impl TaskManager {
    pub(crate) fn new(config: BackgroundTaskConfig) -> Self {
        let system_memory_kb = get_system_memory_kb();
        let (cancel_signals, _) = broadcast::channel(32);

        Self {
            tasks: RwLock::new(HashMap::new()),
            config,
            cancel_signals,
            system_memory_kb,
        }
    }

    pub(crate) fn config(&self) -> &BackgroundTaskConfig {
        &self.config
    }

    pub(crate) fn system_memory_mb(&self) -> u64 {
        self.system_memory_kb / 1024
    }

    pub(crate) async fn available_memory_mb(&self) -> u64 {
        let used_by_tasks = {
            let tasks = self.tasks.read().await;
            let mut total = 0u64;
            for t in tasks.values() {
                total += t.lock().await.memory_usage_mb;
            }
            total
        };
        self.system_memory_mb().saturating_sub(used_by_tasks)
    }

    pub(crate) async fn can_start_task(&self, memory_required_mb: u64) -> bool {
        let active_count = {
            let tasks = self.tasks.read().await;
            let mut count = 0usize;
            for t in tasks.values() {
                if matches!(t.lock().await.status, BackgroundTaskStatus::Running) {
                    count += 1;
                }
            }
            count
        };

        if active_count >= self.config.max_concurrent {
            return false;
        }

        if memory_required_mb > self.config.default_memory_limit_mb {
            return false;
        }

        if memory_required_mb > self.available_memory_mb().await {
            return false;
        }

        true
    }

    pub(crate) async fn create_task(
        &self,
        name: String,
        command: String,
        workdir: PathBuf,
        memory_limit_mb: Option<u64>,
        timeout_seconds: Option<u64>,
    ) -> Result<String, String> {
        let memory_limit = memory_limit_mb.unwrap_or(self.config.default_memory_limit_mb);
        let timeout = timeout_seconds.unwrap_or(self.config.default_timeout_seconds);

        if !self.can_start_task(memory_limit).await {
            return Err(
                "Cannot start task: concurrent limit reached or insufficient memory".to_string(),
            );
        }

        let id = format!("task_{}", uuid_simple());
        let task = BackgroundTask::new(id.clone(), name, command, workdir, memory_limit, timeout);

        let task_arc = Arc::new(Mutex::new(task));
        self.tasks.write().await.insert(id.clone(), task_arc);

        Ok(id)
    }

    pub(crate) async fn start_task(&self, id: &str) -> Result<(), String> {
        let task_arc = {
            let tasks = self.tasks.read().await;
            tasks.get(id).cloned()
        };

        let Some(task_arc) = task_arc else {
            return Err("Task not found".to_string());
        };

        let mut task = task_arc.lock().await;

        if task.status != BackgroundTaskStatus::Pending {
            return Err("Task already started or finished".to_string());
        }

        task.status = BackgroundTaskStatus::Running;
        task.started_at = Some(Instant::now());

        let task_arc_clone = task_arc.clone();
        let cancel_rx = self.cancel_signals.subscribe();
        let workdir = task.workdir.clone();
        let timeout = task.timeout_seconds;

        drop(task);

        tokio::spawn(async move {
            Self::execute_background_task(task_arc_clone, workdir, cancel_rx, timeout).await;
        });

        Ok(())
    }

    async fn execute_background_task(
        task_arc: Arc<Mutex<BackgroundTask>>,
        workdir: PathBuf,
        mut cancel_rx: broadcast::Receiver<String>,
        timeout_seconds: u64,
    ) {
        let task_id = {
            let task = task_arc.lock().await;
            task.id.clone()
        };

        let timeout = Duration::from_secs(timeout_seconds);
        let start = Instant::now();

        let command_str = {
            let task = task_arc.lock().await;
            task.command.clone()
        };

        let parts = shlex::split(&command_str).unwrap_or_default();
        if parts.is_empty() {
            let mut task = task_arc.lock().await;
            task.status = BackgroundTaskStatus::Failed;
            task.stderr_buffer
                .push("Failed to parse command or empty command".to_string());
            return;
        }

        let mut child = match Command::new(&parts[0])
            .args(&parts[1..])
            .current_dir(&workdir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                let mut task = task_arc.lock().await;
                task.status = BackgroundTaskStatus::Failed;
                task.stderr_buffer.push(format!("Failed to spawn: {}", e));
                return;
            }
        };

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let task_arc_stdout = task_arc.clone();
        let task_arc_stderr = task_arc.clone();

        let stdout_handle = if let Some(stdout) = stdout {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            Some(tokio::spawn(async move {
                while let Ok(Some(line)) = lines.next_line().await {
                    let mut task = task_arc_stdout.lock().await;
                    if task.stdout_buffer.len() >= task.max_output_lines {
                        task.stdout_buffer.remove(0);
                    }
                    task.stdout_buffer.push(line);
                }
            }))
        } else {
            None
        };

        let stderr_handle = if let Some(stderr) = stderr {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            Some(tokio::spawn(async move {
                while let Ok(Some(line)) = lines.next_line().await {
                    let mut task = task_arc_stderr.lock().await;
                    if task.stderr_buffer.len() >= task.max_output_lines {
                        task.stderr_buffer.remove(0);
                    }
                    task.stderr_buffer.push(line);
                }
            }))
        } else {
            None
        };

        loop {
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_millis(500)) => {
                    let mut task = task_arc.lock().await;

                    if start.elapsed() > timeout {
                        let _ = child.kill().await;
                        task.status = BackgroundTaskStatus::Failed;
                        task.exit_code = Some(-1);
                        task.stderr_buffer.push("Task timed out".to_string());
                        tracing::info!(task_id = %task_id, timeout_s = %timeout.as_secs(), "job_timeout");
                        break;
                    }

                    task.memory_usage_mb = estimate_process_memory(&mut child).await;

                    if task.is_memory_exceeded() {
                        let _ = child.kill().await;
                        task.status = BackgroundTaskStatus::OOMKilled;
                        task.exit_code = Some(-9);
                        let current_mem = task.memory_usage_mb;
                        let limit_mem = task.memory_limit_mb;
                        task.stderr_buffer.push(format!(
                            "Memory limit exceeded: {}MB > {}MB",
                            current_mem, limit_mem
                        ));
                        tracing::info!(task_id = %task_id, memory_mb = %current_mem, limit_mb = %limit_mem, "job_oom");
                        break;
                    }

                    if task.status != BackgroundTaskStatus::Running {
                        break;
                    }
                }

                result = child.wait() => {
                    let mut task = task_arc.lock().await;
                    match result {
                        Ok(status) => {
                            task.exit_code = status.code();
                            task.status = if status.success() {
                                BackgroundTaskStatus::Completed
                            } else {
                                BackgroundTaskStatus::Failed
                            };
                            tracing::info!(task_id = %task_id, status = %task.status, exit_code = ?task.exit_code, "job_finished");
                        }
                        Err(e) => {
                            task.status = BackgroundTaskStatus::Failed;
                            task.stderr_buffer.push(format!("Process error: {}", e));
                            tracing::info!(task_id = %task_id, error = %e, "job_error");
                        }
                    }
                    break;
                }

                Ok(cancelled_id) = cancel_rx.recv() => {
                    if cancelled_id == task_id {
                        let mut task = task_arc.lock().await;
                        let _ = child.kill().await;
                        task.status = BackgroundTaskStatus::Cancelled;
                        tracing::info!(task_id = %task_id, "job_cancelled");
                        break;
                    }
                }
            }
        }

        if let Some(handle) = stdout_handle {
            let _ = handle.await;
        }
        if let Some(handle) = stderr_handle {
            let _ = handle.await;
        }
    }

    pub(crate) async fn get_task(&self, id: &str) -> Option<BackgroundTask> {
        let tasks = self.tasks.read().await;
        if let Some(t) = tasks.get(id) {
            Some(t.lock().await.clone())
        } else {
            None
        }
    }

    pub(crate) async fn list_tasks(&self) -> Vec<BackgroundTask> {
        let tasks = self.tasks.read().await;
        let mut result = Vec::new();
        for t in tasks.values() {
            result.push(t.lock().await.clone());
        }
        result
    }

    pub(crate) async fn cancel_task(&self, id: &str) -> Result<(), String> {
        let _ = self.cancel_signals.send(id.to_string());
        Ok(())
    }

    pub(crate) async fn remove_task(&self, id: &str) -> Option<BackgroundTask> {
        let task = self.tasks.write().await.remove(id);
        if let Some(t) = task {
            Some(t.lock().await.clone())
        } else {
            None
        }
    }

    pub(crate) async fn clear_completed(&self) {
        let to_remove: Vec<String> = {
            let tasks = self.tasks.read().await;
            let mut ids = Vec::new();
            for (id, t) in tasks.iter() {
                let status = t.lock().await.status;
                if !matches!(
                    status,
                    BackgroundTaskStatus::Running | BackgroundTaskStatus::Pending
                ) {
                    ids.push(id.clone());
                }
            }
            ids
        };
        let mut tasks = self.tasks.write().await;
        for id in &to_remove {
            tasks.remove(id);
        }
    }
}

fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{:x}{:x}", duration.as_secs(), duration.subsec_nanos())
}

fn get_system_memory_kb() -> u64 {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        if let Ok(output) = Command::new("sysctl").arg("-n").arg("hw.memsize").output() {
            if let Ok(s) = String::from_utf8(output.stdout) {
                return s.trim().parse::<u64>().unwrap_or(4_000_000) / 1024;
            }
        }
        4_000_000
    }

    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/proc/meminfo")
            .ok()
            .and_then(|content| {
                content
                    .lines()
                    .find(|l| l.starts_with("MemTotal:"))
                    .and_then(|l| l.split_whitespace().nth(1)?.parse::<u64>().ok())
            })
            .unwrap_or(4_000_000)
    }

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        if let Ok(output) = Command::new("wmic")
            .args(["OS", "get", "TotalVisibleMemorySize", "/value"])
            .output()
        {
            if let Ok(s) = String::from_utf8(output.stdout) {
                return s
                    .lines()
                    .find(|l| l.starts_with("TotalVisibleMemorySize="))
                    .and_then(|l| l.split('=').nth(1)?.trim().parse::<u64>().ok())
                    .unwrap_or(4_000_000);
            }
        }
        4_000_000
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        4_000_000
    }
}

async fn estimate_process_memory(child: &mut Child) -> u64 {
    #[cfg(target_os = "linux")]
    {
        if let Some(pid) = child.id() {
            let status_path = format!("/proc/{}/status", pid);
            if let Ok(content) = std::fs::read_to_string(&status_path) {
                if let Some(line) = content.lines().find(|l| l.starts_with("VmRSS:")) {
                    if let Some(kb) = line.split_whitespace().nth(1) {
                        return kb.parse::<u64>().unwrap_or(0) / 1024;
                    }
                }
            }
        }
    }

    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_status_display() {
        assert_eq!(BackgroundTaskStatus::Running.to_string(), "running");
        assert_eq!(BackgroundTaskStatus::Completed.to_string(), "completed");
        assert_eq!(BackgroundTaskStatus::OOMKilled.to_string(), "oom_killed");
    }

    #[test]
    fn test_task_creation() {
        let task = BackgroundTask::new(
            "test_1".to_string(),
            "test task".to_string(),
            "echo hello".to_string(),
            std::env::temp_dir(),
            1024,
            60,
        );
        assert_eq!(task.status, BackgroundTaskStatus::Pending);
        assert_eq!(task.memory_limit_mb, 1024);
    }

    #[test]
    fn test_memory_exceeded() {
        let mut task = BackgroundTask::new(
            "test_1".to_string(),
            "test".to_string(),
            "echo".to_string(),
            std::env::temp_dir(),
            100,
            60,
        );
        task.memory_usage_mb = 150;
        assert!(task.is_memory_exceeded());
    }

    #[tokio::test]
    async fn test_command_injection_prevention() {
        let config = BackgroundTaskConfig::default();
        let manager = TaskManager::new(config);

        // Generate a path that should NOT be created if the fix works
        let temp_dir = std::env::temp_dir();
        let canary_file = temp_dir.join(format!("injection_canary_{}", uuid_simple()));
        let canary_path = canary_file.to_string_lossy();

        // This command attempts injection. With shlex + direct execution,
        // it should try to execute "echo" with literal arguments like "hello;" and "touch".
        let injection_cmd = format!("echo hello; touch {}", canary_path);

        let id = manager.create_task(
            "injection_test".to_string(),
            injection_cmd.to_string(),
            temp_dir.clone(),
            None,
            None,
        ).await.unwrap();

        manager.start_task(&id).await.unwrap();

        // Wait for task to finish with a bit more patience
        let mut completed = false;
        for _ in 0..50 {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let task = manager.get_task(&id).await.unwrap();
            if task.status != BackgroundTaskStatus::Running && task.status != BackgroundTaskStatus::Pending {
                completed = true;
                break;
            }
        }

        assert!(completed, "Task should have completed or failed");

        // If it was executed via sh -c, the canary file would be created.
        // If it was executed directly, "touch" and the path are just arguments to "echo".
        assert!(!canary_file.exists(), "Injection vulnerability still exists! Canary file created: {}", canary_path);
    }
}
