//! @efficiency-role: util-pure
//! Tool set degradation and retry planning for Task 654.
//!
//! Tracks per-tool failure rates and provides retry strategies
//! with escalation: fast retry -> warn + alternative -> degraded.

use std::collections::HashMap;

/// Record of failures for a single tool.
#[derive(Debug, Clone)]
pub(crate) struct ToolFailureRecord {
    pub(crate) tool_name: String,
    pub(crate) failure_count: u32,
    pub(crate) last_error: Option<String>,
    pub(crate) last_failure: std::time::Instant,
    pub(crate) degraded: bool,
}

/// Retry strategy recommended for a failed tool.
#[derive(Debug, Clone)]
pub(crate) struct RetryPlan {
    pub(crate) should_retry: bool,
    pub(crate) max_retries: u32,
    pub(crate) backoff_base_ms: u64,
    pub(crate) suggested_alternative: Option<String>,
}

/// Plans degradation and retry for tool sets based on failure history.
#[derive(Debug, Clone)]
pub(crate) struct ToolDegradationPlanner {
    records: HashMap<String, ToolFailureRecord>,
    max_failures_before_degrade: u32,
    cooldown_seconds: u64,
}

impl ToolDegradationPlanner {
    pub(crate) fn new() -> Self {
        Self {
            records: HashMap::new(),
            max_failures_before_degrade: 3,
            cooldown_seconds: 30,
        }
    }

    pub(crate) fn with_params(max_failures: u32, cooldown: u64) -> Self {
        Self {
            records: HashMap::new(),
            max_failures_before_degrade: max_failures,
            cooldown_seconds: cooldown,
        }
    }

    pub(crate) fn record_failure(&mut self, tool_name: &str, error: &str) {
        let now = std::time::Instant::now();
        let entry = self
            .records
            .entry(tool_name.to_string())
            .or_insert_with(|| {
                let degraded = false;
                ToolFailureRecord {
                    tool_name: tool_name.to_string(),
                    failure_count: 0,
                    last_error: None,
                    last_failure: now,
                    degraded,
                }
            });
        entry.failure_count += 1;
        entry.last_error = Some(error.to_string());
        entry.last_failure = now;
        entry.degraded = entry.failure_count >= self.max_failures_before_degrade;
    }

    pub(crate) fn record_success(&mut self, tool_name: &str) {
        if let Some(entry) = self.records.get_mut(tool_name) {
            entry.failure_count = 0;
            entry.last_error = None;
            entry.degraded = false;
        }
    }

    pub(crate) fn is_degraded(&self, tool_name: &str) -> bool {
        self.records.get(tool_name).map_or(false, |r| r.degraded)
    }

    pub(crate) fn degraded_tools(&self) -> Vec<&str> {
        self.records
            .iter()
            .filter(|(_, r)| r.degraded)
            .map(|(name, _)| name.as_str())
            .collect()
    }

    pub(crate) fn retry_plan(tool_name: &str, failure_count: u32) -> RetryPlan {
        match failure_count {
            0 | 1 => RetryPlan {
                should_retry: true,
                max_retries: 2,
                backoff_base_ms: 500,
                suggested_alternative: None,
            },
            2 | 3 => RetryPlan {
                should_retry: true,
                max_retries: 1,
                backoff_base_ms: 2000,
                suggested_alternative: Some(format!("consider alternative to {}", tool_name)),
            },
            _ => RetryPlan {
                should_retry: false,
                max_retries: 0,
                backoff_base_ms: 0,
                suggested_alternative: Some(format!(
                    "{} is degraded, use a different tool",
                    tool_name
                )),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_failure_and_degrade() {
        let mut planner = ToolDegradationPlanner::new();
        assert!(!planner.is_degraded("read"));

        planner.record_failure("read", "file not found");
        assert!(!planner.is_degraded("read"));

        planner.record_failure("read", "permission denied");
        assert!(!planner.is_degraded("read"));

        planner.record_failure("read", "timeout");
        assert!(planner.is_degraded("read"));
    }

    #[test]
    fn test_record_success_resets() {
        let mut planner = ToolDegradationPlanner::new();
        planner.record_failure("write", "disk full");
        planner.record_failure("write", "disk full");
        planner.record_failure("write", "disk full");
        assert!(planner.is_degraded("write"));

        planner.record_success("write");
        assert!(!planner.is_degraded("write"));
        assert_eq!(planner.records.get("write").unwrap().failure_count, 0);
    }

    #[test]
    fn test_degraded_tools_list() {
        let mut planner = ToolDegradationPlanner::new();
        planner.record_failure("a", "err");
        planner.record_failure("a", "err");
        planner.record_failure("a", "err");
        planner.record_failure("b", "err");
        planner.record_failure("b", "err");

        let degraded = planner.degraded_tools();
        assert_eq!(degraded, vec!["a"]);
    }

    #[test]
    fn test_retry_plan_low_failures() {
        let plan = ToolDegradationPlanner::retry_plan("read", 0);
        assert!(plan.should_retry);
        assert_eq!(plan.max_retries, 2);
        assert_eq!(plan.backoff_base_ms, 500);
        assert!(plan.suggested_alternative.is_none());

        let plan = ToolDegradationPlanner::retry_plan("read", 1);
        assert!(plan.should_retry);
        assert_eq!(plan.max_retries, 2);
        assert_eq!(plan.backoff_base_ms, 500);
    }

    #[test]
    fn test_retry_plan_medium_failures() {
        let plan = ToolDegradationPlanner::retry_plan("write", 2);
        assert!(plan.should_retry);
        assert_eq!(plan.max_retries, 1);
        assert_eq!(plan.backoff_base_ms, 2000);
        assert!(plan.suggested_alternative.is_some());
        assert!(plan.suggested_alternative.unwrap().contains("write"));

        let plan = ToolDegradationPlanner::retry_plan("write", 3);
        assert!(plan.should_retry);
        assert_eq!(plan.max_retries, 1);
        assert_eq!(plan.backoff_base_ms, 2000);
    }

    #[test]
    fn test_retry_plan_high_failures() {
        let plan = ToolDegradationPlanner::retry_plan("exec", 4);
        assert!(!plan.should_retry);
        assert_eq!(plan.max_retries, 0);
        assert_eq!(plan.backoff_base_ms, 0);
        assert!(plan.suggested_alternative.is_some());

        let plan = ToolDegradationPlanner::retry_plan("exec", 10);
        assert!(!plan.should_retry);
        assert_eq!(plan.max_retries, 0);
    }

    #[test]
    fn test_with_params() {
        let mut planner = ToolDegradationPlanner::with_params(2, 10);
        planner.record_failure("test", "err");
        assert!(!planner.is_degraded("test"));
        planner.record_failure("test", "err");
        assert!(planner.is_degraded("test"));
    }

    #[test]
    fn test_unknown_tool_not_degraded() {
        let planner = ToolDegradationPlanner::new();
        assert!(!planner.is_degraded("nonexistent"));
    }

    #[test]
    fn test_empty_degraded_list() {
        let planner = ToolDegradationPlanner::new();
        assert!(planner.degraded_tools().is_empty());
    }

    #[test]
    fn test_record_failure_updates_last_error() {
        let mut planner = ToolDegradationPlanner::new();
        planner.record_failure("tool", "first error");
        assert_eq!(
            planner.records.get("tool").unwrap().last_error.as_deref(),
            Some("first error")
        );
        planner.record_failure("tool", "second error");
        assert_eq!(
            planner.records.get("tool").unwrap().last_error.as_deref(),
            Some("second error")
        );
    }
}
