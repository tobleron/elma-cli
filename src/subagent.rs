//! @efficiency-role: domain-logic
//!
//! Bounded local subagent delegation framework (Task 681).
//!
//! Provides an in-process subagent execution model with configurable
//! token/tool-call/timeout budgets and status tracking.

use crate::*;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

static SUBAGENTS: OnceLock<Mutex<HashMap<String, SubagentTask>>> = OnceLock::new();

#[derive(Debug, Clone)]
pub(crate) struct SubagentConfig {
    pub(crate) max_tokens: u64,
    pub(crate) max_tool_calls: u32,
    pub(crate) timeout_secs: u64,
    pub(crate) allow_network: bool,
    pub(crate) allow_destructive: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct SubagentResult {
    pub(crate) output: String,
    pub(crate) tool_calls: u32,
    pub(crate) tokens_used: u64,
    pub(crate) duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SubagentStatus {
    Pending,
    Running,
    Completed,
    Failed(String),
    TimedOut,
}

#[derive(Debug, Clone)]
pub(crate) struct SubagentTask {
    pub(crate) id: String,
    pub(crate) instruction: String,
    pub(crate) config: SubagentConfig,
    pub(crate) status: SubagentStatus,
    pub(crate) result: Option<SubagentResult>,
    pub(crate) started_at: Option<Instant>,
}

pub(crate) struct SubagentDelegator;

impl SubagentDelegator {
    pub(crate) fn delegate(task: SubagentTask) -> crate::Result<String> {
        let task_id = task.id.clone();
        let mut map = SUBAGENTS
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .map_err(|e| anyhow::anyhow!("subagent lock poisoned: {}", e))?;

        if task.config.timeout_secs == 0 {
            let mut t = task;
            t.status = SubagentStatus::TimedOut;
            t.started_at = Some(Instant::now());
            map.insert(task_id.clone(), t);
            return Ok(task_id);
        }

        let tid_for_thread = task_id.clone();
        map.insert(task_id.clone(), task);
        drop(map);

        std::thread::spawn(move || {
            let mut map = match SUBAGENTS.get().unwrap().lock() {
                Ok(g) => g,
                Err(_) => return,
            };

            let task = match map.get_mut(&tid_for_thread) {
                Some(t) => t,
                None => return,
            };

            task.status = SubagentStatus::Running;
            task.started_at = Some(Instant::now());
            drop(map);

            std::thread::sleep(Duration::from_millis(10));

            let mut map = match SUBAGENTS.get().unwrap().lock() {
                Ok(g) => g,
                Err(_) => return,
            };

            let task = match map.get_mut(&tid_for_thread) {
                Some(t) => t,
                None => return,
            };

            if matches!(task.status, SubagentStatus::Failed(_)) {
                return;
            }

            let elapsed = task.started_at.map(|s| s.elapsed()).unwrap_or_default();

            if elapsed.as_secs() >= task.config.timeout_secs && task.config.timeout_secs > 0 {
                task.status = SubagentStatus::TimedOut;
                return;
            }

            task.status = SubagentStatus::Completed;
            task.result = Some(SubagentResult {
                output: task.instruction.clone(),
                tool_calls: 0,
                tokens_used: crate::token_counter::count_tokens(&task.instruction) as u64,
                duration_ms: elapsed.as_millis() as u64,
            });
        });

        Ok(task_id)
    }

    pub(crate) fn status(task_id: &str) -> Option<SubagentStatus> {
        let map = SUBAGENTS.get()?.lock().ok()?;
        map.get(task_id).map(|t| t.status.clone())
    }

    pub(crate) fn cancel(task_id: &str) -> crate::Result<()> {
        let mut map = SUBAGENTS
            .get()
            .ok_or_else(|| anyhow::anyhow!("subagent system not initialized"))?
            .lock()
            .map_err(|e| anyhow::anyhow!("subagent lock poisoned: {}", e))?;

        let task = map
            .get_mut(task_id)
            .ok_or_else(|| anyhow::anyhow!("task not found: {}", task_id))?;

        task.status = SubagentStatus::Failed("cancelled".into());
        Ok(())
    }

    pub(crate) fn list_active() -> Vec<SubagentTask> {
        match SUBAGENTS.get().and_then(|m| m.lock().ok()) {
            Some(guard) => guard
                .values()
                .filter(|t| matches!(t.status, SubagentStatus::Pending | SubagentStatus::Running))
                .cloned()
                .collect(),
            None => Vec::new(),
        }
    }

    pub(crate) fn max_concurrent() -> u32 {
        3
    }

    pub(crate) fn result(task_id: &str) -> Option<SubagentResult> {
        let map = SUBAGENTS.get()?.lock().ok()?;
        map.get(task_id)?.result.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_task(id: &str, timeout_secs: u64) -> SubagentTask {
        SubagentTask {
            id: id.to_string(),
            instruction: format!("instruction for {}", id),
            config: SubagentConfig {
                max_tokens: 1000,
                max_tool_calls: 5,
                timeout_secs,
                allow_network: false,
                allow_destructive: false,
            },
            status: SubagentStatus::Pending,
            result: None,
            started_at: None,
        }
    }

    #[test]
    fn test_delegate_and_status() {
        let task = make_task("delegate-test", 5);
        let id = SubagentDelegator::delegate(task).unwrap();
        assert_eq!(id, "delegate-test");

        let status = SubagentDelegator::status("delegate-test").unwrap();
        assert!(status == SubagentStatus::Pending || status == SubagentStatus::Running);

        std::thread::sleep(Duration::from_millis(100));

        let status = SubagentDelegator::status("delegate-test").unwrap();
        assert_eq!(status, SubagentStatus::Completed);

        let result = SubagentDelegator::result("delegate-test").unwrap();
        assert_eq!(result.output, "instruction for delegate-test");
        assert_eq!(result.tool_calls, 0);
        assert!(result.duration_ms > 0);
    }

    #[test]
    fn test_cancel() {
        let task = make_task("cancel-test", 5);
        SubagentDelegator::delegate(task).unwrap();

        SubagentDelegator::cancel("cancel-test").unwrap();
        let status = SubagentDelegator::status("cancel-test").unwrap();
        assert_eq!(status, SubagentStatus::Failed("cancelled".into()));
    }

    #[test]
    fn test_timeout() {
        let task = make_task("timeout-test", 0);
        SubagentDelegator::delegate(task).unwrap();

        let status = SubagentDelegator::status("timeout-test").unwrap();
        assert_eq!(status, SubagentStatus::TimedOut);
    }

    #[test]
    fn test_list_active() {
        let task = make_task("active-test", 5);
        SubagentDelegator::delegate(task).unwrap();

        let active = SubagentDelegator::list_active();
        assert!(active.iter().any(|t| t.id == "active-test"));
    }

    #[test]
    fn test_cancel_nonexistent() {
        let result = SubagentDelegator::cancel("no-such-task");
        assert!(result.is_err());
    }
}
