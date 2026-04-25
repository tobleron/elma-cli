//! @efficiency-role: domain-logic
//!
//! Unified stop policy with explicit stop reasons, stage-aware budgets,
//! and user-visible explanations. Absorbs the old MAX_TOOL_ITERATIONS
//! and stagnation logic from tool_loop.rs into a single enforcement point.

use crate::*;
use std::collections::HashSet;
use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum StopReason {
    StageBudgetExceeded,
    TaskBudgetExceeded,
    RepeatedToolFailure,
    RepeatedNoNewEvidence,
    RepeatedSameCommand,
    RepeatedSameConclusion,
    WallClockExceeded,
    ModelProgressStalled,
    UserInterrupted,
}

impl StopReason {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            StopReason::StageBudgetExceeded => "stage_budget_exceeded",
            StopReason::TaskBudgetExceeded => "task_budget_exceeded",
            StopReason::RepeatedToolFailure => "repeated_tool_failure",
            StopReason::RepeatedNoNewEvidence => "repeated_no_new_evidence",
            StopReason::RepeatedSameCommand => "repeated_same_command",
            StopReason::RepeatedSameConclusion => "repeated_same_conclusion",
            StopReason::WallClockExceeded => "wall_clock_exceeded",
            StopReason::ModelProgressStalled => "model_progress_stalled",
            StopReason::UserInterrupted => "user_interrupted",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct StageBudget {
    pub max_tool_calls: usize,
    pub max_iterations: usize,
    pub max_repeated_failures: usize,
    pub max_stagnation_cycles: usize,
    pub max_wall_clock_s: u64,
}

impl Default for StageBudget {
    fn default() -> Self {
        Self {
            max_tool_calls: 30,
            max_iterations: 15,
            max_repeated_failures: 3,
            max_stagnation_cycles: 3,
            max_wall_clock_s: 300,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct StopOutcome {
    pub reason: StopReason,
    pub stage_index: usize,
    pub stage_skill: String,
    pub summary: String,
    pub next_step_hint: String,
}

/// Central stop-policy tracker. Created once per tool-loop execution and
/// queried before each iteration to decide whether execution must halt.
pub(crate) struct StopPolicy {
    budget: StageBudget,
    iteration: usize,
    total_tool_calls: usize,
    stagnation_runs: usize,
    start_time: Instant,
    seen_signals: HashSet<String>,
    tool_failures: Vec<(String, String)>,
    recent_commands: Vec<String>,
    stage_index: usize,
    stage_skill: String,
}

impl StopPolicy {
    pub(crate) fn new(budget: StageBudget) -> Self {
        Self {
            budget,
            iteration: 0,
            total_tool_calls: 0,
            stagnation_runs: 0,
            start_time: Instant::now(),
            seen_signals: HashSet::new(),
            tool_failures: Vec::new(),
            recent_commands: Vec::new(),
            stage_index: 0,
            stage_skill: "general".to_string(),
        }
    }

    pub(crate) fn with_stage(mut self, index: usize, skill: &str) -> Self {
        self.stage_index = index;
        self.stage_skill = skill.to_string();
        self
    }

    /// Call at the top of each iteration. Returns a stop outcome if a
    /// budget has been exceeded before the iteration starts.
    pub(crate) fn start_iteration(&mut self) -> Option<StopOutcome> {
        self.iteration += 1;

        if self.iteration > self.budget.max_iterations {
            return Some(self.build_outcome(
                StopReason::StageBudgetExceeded,
                "Iteration budget exhausted. The model has used the maximum number of tool loops for this stage.",
            ));
        }

        let elapsed = self.start_time.elapsed().as_secs();
        if elapsed > self.budget.max_wall_clock_s {
            return Some(self.build_outcome(
                StopReason::WallClockExceeded,
                "Wall-clock budget exhausted. The stage has run longer than the configured maximum.",
            ));
        }

        None
    }

    /// Record that tool calls were observed this iteration.
    pub(crate) fn record_tool_calls(&mut self, calls: &[ToolCall]) -> Option<StopOutcome> {
        self.total_tool_calls += calls.len();

        if self.total_tool_calls > self.budget.max_tool_calls {
            return Some(self.build_outcome(
                StopReason::StageBudgetExceeded,
                "Tool-call budget exhausted. The model has issued more tool calls than allowed for this stage.",
            ));
        }

        // Track shell commands for repeated-command detection
        for tc in calls {
            if tc.function.name == "shell" {
                if let Ok(args) = serde_json::from_str::<serde_json::Value>(&tc.function.arguments)
                {
                    if let Some(cmd) = args.get("command").and_then(|v| v.as_str()) {
                        let normalized = normalize_shell_signal(cmd);
                        self.recent_commands.push(normalized);
                    }
                }
            }
        }

        if self.recent_commands.len() >= 3 {
            let last_three = &self.recent_commands[self.recent_commands.len() - 3..];
            if last_three.windows(2).all(|w| w[0] == w[1]) {
                return Some(self.build_outcome(
                    StopReason::RepeatedSameCommand,
                    "The same shell command was repeated multiple times without changing scope. A narrower query or manual inspection may be needed.",
                ));
            }
        }

        None
    }

    /// Record the result of a single tool execution.
    pub(crate) fn record_tool_result(
        &mut self,
        call: &ToolCall,
        result: &crate::tool_calling::ToolExecutionResult,
    ) {
        if !result.ok {
            let mut error_class = classify_error(&result.content);
            if call.function.name == "shell" {
                if let Ok(args) =
                    serde_json::from_str::<serde_json::Value>(&call.function.arguments)
                {
                    if let Some(cmd) = args.get("command").and_then(|v| v.as_str()) {
                        error_class = format!("{}_{}", error_class, classify_command_strategy(cmd));
                    }
                }
            }
            self.tool_failures
                .push((call.function.name.clone(), error_class));
        }
    }

    /// Call when no new tool signals were seen this iteration (stagnation).
    pub(crate) fn record_stagnation(&mut self) -> Option<StopOutcome> {
        self.stagnation_runs += 1;
        if self.stagnation_runs >= self.budget.max_stagnation_cycles {
            return Some(self.build_outcome(
                StopReason::RepeatedNoNewEvidence,
                "Stagnation threshold reached. The model is repeating the same tool calls without producing new evidence.",
            ));
        }
        None
    }

    /// Call when new tool signals *were* seen this iteration.
    pub(crate) fn record_new_signals(&mut self) {
        self.stagnation_runs = 0;
    }

    /// Register a tool signal so the policy knows whether future calls are novel.
    pub(crate) fn register_signal(&mut self, signal: String) -> bool {
        self.seen_signals.insert(signal)
    }

    /// General check that can be called at any safe point.
    pub(crate) fn check_should_stop(&self) -> Option<StopOutcome> {
        if self.iteration > self.budget.max_iterations {
            return Some(self.build_outcome(
                StopReason::StageBudgetExceeded,
                "Iteration budget exhausted.",
            ));
        }

        if self.total_tool_calls > self.budget.max_tool_calls {
            return Some(self.build_outcome(
                StopReason::StageBudgetExceeded,
                "Tool-call budget exhausted.",
            ));
        }

        if self.stagnation_runs >= self.budget.max_stagnation_cycles {
            return Some(self.build_outcome(
                StopReason::RepeatedNoNewEvidence,
                "Stagnation threshold reached.",
            ));
        }

        let elapsed = self.start_time.elapsed().as_secs();
        if elapsed > self.budget.max_wall_clock_s {
            return Some(self.build_outcome(
                StopReason::WallClockExceeded,
                "Wall-clock budget exhausted.",
            ));
        }

        if self.max_consecutive_failures() >= self.budget.max_repeated_failures {
            return Some(self.build_outcome(
                StopReason::RepeatedToolFailure,
                "The same tool family and strategy has failed repeatedly. Check permissions, paths, or command syntax.",
            ));
        }

        None
    }

    /// Build a user-interrupt outcome.
    pub(crate) fn user_interrupt(&self) -> StopOutcome {
        self.build_outcome(StopReason::UserInterrupted, "Stopped by user interrupt.")
    }

    fn max_consecutive_failures(&self) -> usize {
        if self.tool_failures.is_empty() {
            return 0;
        }
        let mut max_count = 0;
        let mut current_count = 0;
        let mut current_fail = &self.tool_failures[0];

        for f in &self.tool_failures {
            if f == current_fail {
                current_count += 1;
            } else {
                current_fail = f;
                current_count = 1;
            }
            if current_count > max_count {
                max_count = current_count;
            }
        }
        max_count
    }

    pub(crate) fn iteration(&self) -> usize {
        self.iteration
    }

    pub(crate) fn total_tool_calls(&self) -> usize {
        self.total_tool_calls
    }

    pub(crate) fn stagnation_runs(&self) -> usize {
        self.stagnation_runs
    }

    pub(crate) fn max_iterations(&self) -> usize {
        self.budget.max_iterations
    }

    fn build_outcome(&self, reason: StopReason, summary: impl Into<String>) -> StopOutcome {
        let hint = next_step_hint(&reason);
        StopOutcome {
            reason,
            stage_index: self.stage_index,
            stage_skill: self.stage_skill.clone(),
            summary: summary.into(),
            next_step_hint: hint,
        }
    }
}

fn next_step_hint(reason: &StopReason) -> String {
    match reason {
        StopReason::StageBudgetExceeded => {
            "Narrow the request scope or split the work into smaller sequential stages.".to_string()
        }
        StopReason::TaskBudgetExceeded => {
            "Break the task into independent sub-tasks and run them one at a time.".to_string()
        }
        StopReason::RepeatedToolFailure => {
            "Verify the command or path manually, then retry with corrected input.".to_string()
        }
        StopReason::RepeatedNoNewEvidence => {
            "Ask a more specific question or provide a file path to inspect directly.".to_string()
        }
        StopReason::RepeatedSameCommand => {
            "Change the query parameters or inspect a different file/directory.".to_string()
        }
        StopReason::RepeatedSameConclusion => {
            "Introduce a new evidence source or rephrase the objective.".to_string()
        }
        StopReason::WallClockExceeded => {
            "Run the step again with a tighter scope, or split it into smaller chunks.".to_string()
        }
        StopReason::ModelProgressStalled => {
            "Restart the turn with a simpler, more direct prompt.".to_string()
        }
        StopReason::UserInterrupted => "Resume with a refined objective when ready.".to_string(),
    }
}

/// Lightweight error classification for repeated-failure detection.
fn classify_error(content: &str) -> String {
    let lower = content.to_ascii_lowercase();
    if lower.contains("permission denied") || lower.contains("access denied") {
        "permission_denied".to_string()
    } else if lower.contains("no such file") || lower.contains("not found") {
        "not_found".to_string()
    } else if lower.contains("command not found") || lower.contains("unknown command") {
        "command_not_found".to_string()
    } else if lower.contains("timeout") || lower.contains("timed out") {
        "timeout".to_string()
    } else {
        "other".to_string()
    }
}

/// Classify the command strategy to allow bounded fallbacks when switching approaches
pub(crate) fn classify_command_strategy(cmd: &str) -> String {
    let lower = cmd.to_ascii_lowercase();
    if lower.contains("stat ") && lower.contains("while read") {
        "stat_loop".to_string()
    } else if lower.contains("stat ") && lower.contains("find ") {
        "find_stat".to_string()
    } else if lower.contains("find ") && lower.contains("du -") {
        "find_du_aggregate".to_string()
    } else if lower.contains("du -") {
        "du_aggregate".to_string()
    } else if lower.contains("find ") && lower.contains("-mtime") {
        "find_mtime".to_string()
    } else if lower.contains("find ") && lower.contains("-ls") {
        "find_ls".to_string()
    } else if lower.contains("find ") && lower.contains("wc -") {
        "find_count".to_string()
    } else if lower.contains("find ") {
        "find_other".to_string()
    } else {
        "other_shell".to_string()
    }
}

/// Normalize a shell command string for repeated-command detection.
/// Collapses highly variable identifiers (timestamps, session ids) so repeated
/// directory-probing loops are detected as the same strategy.
pub(crate) fn normalize_shell_signal(cmd: &str) -> String {
    let mut out = String::with_capacity(cmd.len());
    let mut prev_was_digit = false;
    for ch in cmd.chars() {
        if ch.is_ascii_digit() {
            if !prev_was_digit {
                out.push('#');
                prev_was_digit = true;
            }
            continue;
        }
        prev_was_digit = false;
        out.push(ch);
    }
    out.replace("s_#_#", "s_SESSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_budget_matches_legacy_constants() {
        let b = StageBudget::default();
        assert_eq!(b.max_iterations, 15);
        assert_eq!(b.max_stagnation_cycles, 3);
    }

    #[test]
    fn iteration_limit_triggers_stop() {
        let budget = StageBudget {
            max_iterations: 2,
            ..Default::default()
        };
        let mut policy = StopPolicy::new(budget);
        assert!(policy.start_iteration().is_none()); // iter 1
        assert!(policy.start_iteration().is_none()); // iter 2
        assert!(policy.start_iteration().is_some()); // iter 3 -> stop
    }

    #[test]
    fn stagnation_limit_triggers_stop() {
        let budget = StageBudget {
            max_stagnation_cycles: 2,
            ..Default::default()
        };
        let mut policy = StopPolicy::new(budget);
        policy.start_iteration();
        assert!(policy.record_stagnation().is_none()); // run 1
        assert!(policy.record_stagnation().is_some()); // run 2 -> stop
    }

    #[test]
    fn new_signals_reset_stagnation() {
        let budget = StageBudget {
            max_stagnation_cycles: 2,
            ..Default::default()
        };
        let mut policy = StopPolicy::new(budget);
        policy.start_iteration();
        policy.record_stagnation();
        policy.record_new_signals();
        policy.record_stagnation();
        assert!(policy.check_should_stop().is_none());
    }

    #[test]
    fn tool_call_limit_triggers_stop() {
        let budget = StageBudget {
            max_tool_calls: 2,
            ..Default::default()
        };
        let mut policy = StopPolicy::new(budget);
        let calls = vec![
            ToolCall {
                id: "c1".to_string(),
                call_type: "function".to_string(),
                function: ToolFunctionCall {
                    name: "read".to_string(),
                    arguments: r#"{"path":"a"}"#.to_string(),
                },
            },
            ToolCall {
                id: "c2".to_string(),
                call_type: "function".to_string(),
                function: ToolFunctionCall {
                    name: "read".to_string(),
                    arguments: r#"{"path":"b"}"#.to_string(),
                },
            },
            ToolCall {
                id: "c3".to_string(),
                call_type: "function".to_string(),
                function: ToolFunctionCall {
                    name: "read".to_string(),
                    arguments: r#"{"path":"c"}"#.to_string(),
                },
            },
        ];
        assert!(policy.record_tool_calls(&calls[..2]).is_none());
        assert!(policy.record_tool_calls(&calls[2..]).is_some());
    }

    #[test]
    fn repeated_same_command_triggers_stop() {
        let budget = StageBudget {
            max_stagnation_cycles: 5, // high so stagnation doesn't fire first
            ..Default::default()
        };
        let mut policy = StopPolicy::new(budget);
        let cmd = ToolCall {
            id: "c1".to_string(),
            call_type: "function".to_string(),
            function: ToolFunctionCall {
                name: "shell".to_string(),
                arguments: r#"{"command":"ls src"}"#.to_string(),
            },
        };
        policy.record_tool_calls(&[cmd.clone()]);
        policy.record_tool_calls(&[cmd.clone()]);
        assert!(policy.record_tool_calls(&[cmd]).is_some());
    }

    #[test]
    fn stop_reason_serializes_cleanly() {
        let r = StopReason::RepeatedNoNewEvidence;
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("RepeatedNoNewEvidence"));
    }

    #[test]
    fn user_interrupt_outcome_has_hint() {
        let budget = StageBudget::default();
        let policy = StopPolicy::new(budget);
        let outcome = policy.user_interrupt();
        assert_eq!(outcome.reason, StopReason::UserInterrupted);
        assert!(!outcome.next_step_hint.is_empty());
    }

    #[test]
    fn strategy_change_resets_repeated_failure_count() {
        let budget = StageBudget {
            max_repeated_failures: 2,
            ..Default::default()
        };
        let mut policy = StopPolicy::new(budget);

        let mut fail_result = crate::tool_calling::ToolExecutionResult {
            tool_call_id: "c1".to_string(),
            tool_name: "shell".to_string(),
            content: "command not found".to_string(),
            ok: false,
        };

        // Strategy 1 fails
        let call1 = ToolCall {
            id: "c1".to_string(),
            call_type: "function".to_string(),
            function: ToolFunctionCall {
                name: "shell".to_string(),
                arguments: r#"{"command":"find . | while read f; do stat $f; done"}"#.to_string(),
            },
        };
        fail_result.tool_call_id = "c1".to_string();
        policy.record_tool_result(&call1, &fail_result);
        assert!(
            policy.check_should_stop().is_none(),
            "Should not stop on first failure"
        );

        // Strategy 2 fails (different strategy class)
        let call2 = ToolCall {
            id: "c2".to_string(),
            call_type: "function".to_string(),
            function: ToolFunctionCall {
                name: "shell".to_string(),
                arguments: r#"{"command":"du -sh ."}"#.to_string(),
            },
        };
        fail_result.tool_call_id = "c2".to_string();
        policy.record_tool_result(&call2, &fail_result);
        assert!(
            policy.check_should_stop().is_none(),
            "Should not stop, strategy changed"
        );

        // Strategy 2 fails again (same strategy class)
        let call3 = ToolCall {
            id: "c3".to_string(),
            call_type: "function".to_string(),
            function: ToolFunctionCall {
                name: "shell".to_string(),
                arguments: r#"{"command":"du -sh src"}"#.to_string(),
            },
        };
        fail_result.tool_call_id = "c3".to_string();
        policy.record_tool_result(&call3, &fail_result);
        assert!(
            policy.check_should_stop().is_some(),
            "Should stop, Strategy 2 failed twice"
        );
    }

    #[test]
    fn test_classify_command_strategy() {
        // macOS / BSD style aggregate
        assert_eq!(
            classify_command_strategy("find . -type f -mtime +14 -exec du -h {} +"),
            "find_du_aggregate"
        );

        // Linux / GNU style date
        assert_eq!(
            classify_command_strategy("date -d '@123456'"),
            "other_shell"
        );

        // stat loop
        assert_eq!(
            classify_command_strategy("find . -type f | while read f; do stat $f; done"),
            "stat_loop"
        );

        // du aggregate
        assert_eq!(
            classify_command_strategy("du -sh sessions/"),
            "du_aggregate"
        );

        // find count
        assert_eq!(
            classify_command_strategy("find sessions -type f | wc -l"),
            "find_count"
        );

        // other
        assert_eq!(classify_command_strategy("ls -la"), "other_shell");
    }
}
