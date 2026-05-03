//! @efficiency-role: domain-logic
//!
//! Unified stop policy with explicit stop reasons, stage-aware budgets,
//! and user-visible explanations. Absorbs the old MAX_TOOL_ITERATIONS
//! and stagnation logic from tool_loop.rs into a single enforcement point.

use crate::*;
use std::collections::HashSet;
use std::time::Instant;

/// Truncate a tool call's arguments for inclusion in stagnation trace output.
fn truncate_tool_args(call: &ToolCall) -> String {
    let args_str = &call.function.arguments;
    if args_str.len() <= 80 {
        args_str.to_string()
    } else {
        format!("{}...", &args_str[..77])
    }
}

/// Compute a short hash of tool arguments for change-detection.
fn args_hash(args: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    args.hash(&mut hasher);
    hasher.finish()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum StopReason {
    StageBudgetExceeded,
    IterationLimitReached,
    TaskBudgetExceeded,
    RepeatedToolFailure,
    RepeatedNoNewEvidence,
    RepeatedSameCommand,
    RepeatedSameConclusion,
    RespondAbuse,
    RespondOnlyStagnation,
    WallClockExceeded,
    ModelProgressStalled,
    UserInterrupted,
}

impl StopReason {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            StopReason::StageBudgetExceeded => "stage_budget_exceeded",
            StopReason::IterationLimitReached => "iteration_limit_reached",
            StopReason::TaskBudgetExceeded => "task_budget_exceeded",
            StopReason::RepeatedToolFailure => "repeated_tool_failure",
            StopReason::RepeatedNoNewEvidence => "repeated_no_new_evidence",
            StopReason::RepeatedSameCommand => "repeated_same_command",
            StopReason::RepeatedSameConclusion => "repeated_same_conclusion",
            StopReason::RespondAbuse => "respond_abuse",
            StopReason::RespondOnlyStagnation => "respond_only_stagnation",
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
            max_tool_calls: 0,
            max_iterations: 20, // Default cap for safety
            max_repeated_failures: 6,
            max_stagnation_cycles: 8,
            max_wall_clock_s: 300,
        }
    }
}

impl StageBudget {
    pub(crate) fn from_complexity(complexity: &str) -> Self {
        let max_iterations = match complexity.to_ascii_uppercase().as_str() {
            "DIRECT" => 3,
            "INVESTIGATE" => 6,
            "MULTISTEP" => 12,
            "OPEN_ENDED" => 20,
            _ => 6, // Default for unknown complexity
        };
        Self {
            max_iterations,
            ..Self::default()
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
    /// Track last failed tool name for argument-change detection.
    last_failed_tool_name: Option<String>,
    /// Hash of last failed tool's arguments (for change detection).
    last_failed_args_hash: Option<u64>,
    /// Stubborn attempts count: retries with same tool name but different args.
    stubborn_attempts: usize,
    recent_commands: Vec<String>,
    stage_index: usize,
    stage_skill: String,
    // T303: Retry loop detection
    consecutive_shell_failures: usize,
    last_shell_strategy: Option<String>,
    last_shell_scope: Option<String>,
    retry_loop_detected: bool,
    last_error_class: Option<String>,
    // T333: Respond abuse guard
    consecutive_respond_calls: usize,
    consecutive_respond_only_turns: usize,
    has_real_tool_calls_this_turn: bool,
    // Goal consistency: track when we last checked at 18-tool-call milestones
    last_milestone_checked: usize,
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
            last_failed_tool_name: None,
            last_failed_args_hash: None,
            stubborn_attempts: 0,
            recent_commands: Vec::new(),
            stage_index: 0,
            stage_skill: "general".to_string(),
            consecutive_shell_failures: 0,
            last_shell_strategy: None,
            last_shell_scope: None,
            retry_loop_detected: false,
            last_error_class: None,
            consecutive_respond_calls: 0,
            consecutive_respond_only_turns: 0,
            has_real_tool_calls_this_turn: false,
            last_milestone_checked: 0,
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
        self.has_real_tool_calls_this_turn = false;

        if self.budget.max_iterations > 0 && self.iteration > self.budget.max_iterations {
            return Some(self.build_outcome(
                StopReason::IterationLimitReached,
                format!("Iteration limit reached ({}/{}). The model has used the maximum number of tool loops allowed for this complexity tier.", self.iteration - 1, self.budget.max_iterations),
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

        if self.budget.max_tool_calls > 0 && self.total_tool_calls > self.budget.max_tool_calls {
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
                        let normalized = crate::text_utils::normalize_shell_signal(cmd);
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
    /// T303: Tracks shell failures for retry-loop detection.
    pub(crate) fn record_tool_result(
        &mut self,
        call: &ToolCall,
        result: &crate::tool_calling::ToolExecutionResult,
    ) {
        if !result.ok {
            let mut error_class = classify_error(result);
            if call.function.name == "shell" {
                if let Ok(args) =
                    serde_json::from_str::<serde_json::Value>(&call.function.arguments)
                {
                    if let Some(cmd) = args.get("command").and_then(|v| v.as_str()) {
                        error_class = format!("{}_{}", error_class, classify_command_strategy(cmd));
                        self.record_shell_failure(cmd, &error_class);
                    }
                }
            }
            self.tool_failures
                .push((call.function.name.clone(), error_class));

            // Track argument-level changes for non-shell failures only.
            // Shell failures have their own strategy-based tracking via record_shell_failure.
            if call.function.name != "shell" {
                let current_args_hash = args_hash(&call.function.arguments);
                match &self.last_failed_tool_name {
                    Some(last_name) if *last_name == call.function.name => {
                        if self.last_failed_args_hash != Some(current_args_hash) {
                            // Arguments changed — use stubborn counter, remove recent failure to buy time
                            self.stubborn_attempts += 1;
                            if self.stubborn_attempts <= self.budget.max_repeated_failures {
                                self.tool_failures.pop();
                            }
                        } else {
                            self.stubborn_attempts = 0;
                        }
                    }
                    _ => {
                        self.stubborn_attempts = 0;
                    }
                }
                self.last_failed_tool_name = Some(call.function.name.clone());
                self.last_failed_args_hash = Some(current_args_hash);
            }
        } else if call.function.name == "shell" {
            // Reset consecutive failure count on success
            self.consecutive_shell_failures = 0;
            self.last_shell_strategy = None;
            self.last_shell_scope = None;
        }
    }

    /// Get the name of the last failed tool (for stagnation trace).
    pub(crate) fn last_failed_tool_signal(&self) -> String {
        if let Some((tool, _)) = self.tool_failures.last() {
            tool.clone()
        } else {
            String::new()
        }
    }

    /// T303: Track shell failures for retry-loop detection.
    /// Detects when the model retries the same strategy with same or widening scope.
    fn record_shell_failure(&mut self, cmd: &str, error_class: &str) {
        let strategy = classify_command_strategy(cmd);
        let scope = estimate_command_scope(cmd);
        self.last_error_class = Some(error_class.to_string());

        // Check if this is a retry of the same strategy
        if let Some(ref last_strategy) = self.last_shell_strategy {
            if *last_strategy == strategy {
                // Same strategy — check if scope is same or widening
                if let Some(ref last_scope) = self.last_shell_scope {
                    // Scope is widening or same if it's equal or larger
                    // (simplified: treat same strategy as retry regardless of scope)
                    self.consecutive_shell_failures += 1;

                    // Detect retry loop: 3+ consecutive failures with same strategy
                    if self.consecutive_shell_failures >= 3 && !self.retry_loop_detected {
                        self.retry_loop_detected = true;
                    }
                } else {
                    self.consecutive_shell_failures += 1;
                }
            } else {
                // Strategy changed — reset counter
                self.consecutive_shell_failures = 1;
            }
        } else {
            self.consecutive_shell_failures = 1;
        }

        self.last_shell_strategy = Some(strategy);
        self.last_shell_scope = Some(scope);
    }

    /// T303: Check if a retry loop has been detected.
    /// Returns true if ≥3 consecutive shell failures with same strategy.
    pub(crate) fn is_retry_loop_detected(&self) -> bool {
        self.retry_loop_detected
    }

    /// T306: Check if the model is struggling and decomposition may help.
    /// Returns true if repeated failures, stagnation, or other struggle indicators.
    pub(crate) fn is_struggling(&self) -> bool {
        self.tool_failures.len() >= self.budget.max_repeated_failures
            || self.stagnation_runs >= self.budget.max_stagnation_cycles
            || self.retry_loop_detected
    }

    /// T303: Get the count of consecutive shell failures.
    pub(crate) fn consecutive_shell_failures(&self) -> usize {
        self.consecutive_shell_failures
    }

    /// T303: Generate a strategy-shift hint when retry loop detected.
    pub(crate) fn strategy_shift_hint(&self) -> Option<String> {
        if !self.retry_loop_detected {
            return None;
        }
        let strategy = self.last_shell_strategy.as_deref().unwrap_or("unknown");
        let error_class = self.last_error_class.as_deref().unwrap_or("unknown");
        let scope = self.last_shell_scope.as_deref().unwrap_or("unknown");
        let alternatives = suggest_alternatives(strategy, error_class, scope);
        Some(format!(
            "⚠️ Strategy Retry Detected: The same shell strategy ('{}') has failed {} times consecutively.\n\n{}

Consider: (1) using a different tool (read/search instead of shell), (2) narrowing the scope with -maxdepth or specific paths, (3) breaking the task into smaller steps, or (4) asking the user about directory structure.",
            strategy,
            self.consecutive_shell_failures,
            alternatives
        ))
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

    /// Trace info about stagnation for debugging (tool + args).
    pub(crate) fn stagnation_trace_info(&self) -> String {
        let tool = self.last_failed_tool_signal();
        if tool.is_empty() {
            format!("stagnation run {} (tool: unknown)", self.stagnation_runs)
        } else {
            format!("stagnation run {} (tool: {})", self.stagnation_runs, tool)
        }
    }

    /// Call when new tool signals *were* seen this iteration.
    pub(crate) fn record_new_signals(&mut self) {
        self.stagnation_runs = 0;
    }

    /// Register a tool signal so the policy knows whether future calls are novel.
    pub(crate) fn register_signal(&mut self, signal: String) -> bool {
        self.seen_signals.insert(signal)
    }

    /// Reset all signal history for a new user turn.
    /// Prevents signals from a previous turn or fallback loop poisoning the next.
    pub(crate) fn reset_signals(&mut self) {
        self.seen_signals.clear();
        self.stagnation_runs = 0;
        self.consecutive_respond_calls = 0;
        self.consecutive_respond_only_turns = 0;
        self.last_failed_tool_name = None;
    }

    // ── T333: Respond abuse guard ──

    /// Increment the consecutive respond counter. Called each time `respond` is executed.
    pub(crate) fn increment_respond_counter(&mut self) {
        self.consecutive_respond_calls += 1;
    }

    /// Get the current consecutive respond count.
    pub(crate) fn consecutive_respond_calls(&self) -> usize {
        self.consecutive_respond_calls
    }

    /// Reset the respond counter. Called when a real (evidence-collecting) tool runs.
    pub(crate) fn reset_respond_counter(&mut self) {
        self.consecutive_respond_calls = 0;
        self.consecutive_respond_only_turns = 0;
    }

    /// Mark that a real tool call (non-respond, non-meta) was seen this turn.
    pub(crate) fn mark_real_tool_call(&mut self) {
        self.has_real_tool_calls_this_turn = true;
    }

    /// Whether any real tool call has been seen this turn.
    pub(crate) fn has_real_tool_calls_this_turn(&self) -> bool {
        self.has_real_tool_calls_this_turn
    }

    /// Increment respond-only turn counter. Returns a stop outcome if ≥5 respond-only turns.
    pub(crate) fn record_respond_only_turn(&mut self) -> Option<StopOutcome> {
        self.consecutive_respond_only_turns += 1;
        if self.consecutive_respond_only_turns >= 5 {
            return Some(self.build_outcome(
                StopReason::RespondAbuse,
                "Respond Abuse: The model has called 'respond' 5+ times without using any evidence-collecting tools. This usually indicates the model is stuck in a conversational loop.",
            ));
        }
        None
    }

    /// Check respond-only stagnation. Separate from main stagnation to avoid
    /// the empty-string signal loophole.
    pub(crate) fn check_respond_only_stagnation(&mut self) -> Option<StopOutcome> {
        if self.consecutive_respond_only_turns >= 5 {
            return Some(self.build_outcome(
                StopReason::RespondOnlyStagnation,
                "Respond-only stagnation: the model called respond 5+ times without real evidence collection.",
            ));
        }
        None
    }

    /// Get total tool failures recorded.
    pub(crate) fn tool_failures_count(&self) -> usize {
        self.tool_failures.len()
    }

    /// General check that can be called at any safe point.
    pub(crate) fn check_should_stop(&self) -> Option<StopOutcome> {
        if self.budget.max_iterations > 0 && self.iteration > self.budget.max_iterations {
            return Some(self.build_outcome(
                StopReason::StageBudgetExceeded,
                "Iteration budget exhausted.",
            ));
        }

        if self.budget.max_tool_calls > 0 && self.total_tool_calls > self.budget.max_tool_calls {
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

    /// Returns true every 18 tool calls (18, 36, 54, …) — only once per milestone.
    /// The caller must call this after each batch of tool calls is recorded.
    /// Resets the internal milestone tracker so the same milestone won't fire twice.
    pub(crate) fn goal_consistency_check_needed(&mut self) -> bool {
        if self.total_tool_calls == 0 {
            return false;
        }
        let current_milestone = self.total_tool_calls / 18;
        if current_milestone > self.last_milestone_checked {
            self.last_milestone_checked = current_milestone;
            true
        } else {
            false
        }
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
        StopReason::IterationLimitReached => {
            "The hard iteration cap was reached. Consider breaking the task into smaller sub-tasks.".to_string()
        }
        StopReason::TaskBudgetExceeded => {
            "Break the task into independent sub-tasks and run them one at a time.".to_string()
        }
        StopReason::RepeatedToolFailure => {
            "The same tool failed multiple times. Consider switching to an alternative tool: use 'shell' with cat/head for file reading, or 'search' with grep for finding content, or 'glob' for filename matching.".to_string()
        }
        StopReason::RepeatedNoNewEvidence => {
            "The model is stuck repeating tool calls. Try a different approach or ask for missing information.".to_string()
        }
        StopReason::RepeatedSameCommand => {
            "Change the query parameters or inspect a different file/directory.".to_string()
        }
        StopReason::RepeatedSameConclusion => {
            "Introduce a new evidence source or rephrase the objective.".to_string()
        }
        StopReason::RespondAbuse => {
            "The model is stuck in a conversational loop. Use search, read, or shell tools to collect evidence.".to_string()
        }
        StopReason::RespondOnlyStagnation => {
            "Status updates alone do not solve tasks. Gather evidence from the workspace.".to_string()
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

/// Estimate command scope: "narrow", "medium", or "wide".
/// Used to detect widening scope in retry loops.
fn estimate_command_scope(cmd: &str) -> String {
    let lower = cmd.to_ascii_lowercase();
    // Narrow: has maxdepth, specific path, single file
    if lower.contains("-maxdepth") || lower.contains("head -") || lower.contains("tail -") {
        return "narrow".to_string();
    }
    // Wide: no exclusions, recursive find without filters
    if lower.contains("find ") && !lower.contains("! -path") && !lower.contains("-name") {
        return "wide".to_string();
    }
    // Medium: has some filters but not maxdepth
    "medium".to_string()
}

/// Suggest alternative strategies based on error class and scope.
/// Principle-based: describes what went wrong and recovery principles,
/// rather than listing hardcoded alternative commands.
fn suggest_alternatives(failed_strategy: &str, error_class: &str, scope: &str) -> String {
    let error_guidance = match error_class {
        "timeout" | "killed_signal_9" | "killed_signal_15" =>
            "This command exceeded time/memory limits. Consider: narrowing the scope with specific paths or -maxdepth, breaking into per-directory steps, or using a lighter-weight tool like read/search for known files.",
        "permission_denied" =>
            "This command hit a permission barrier. Consider: targeting specific accessible directories, or using read tool for known files instead of broad shell scans.",
        "not_found" | "no_such_file" =>
            "The target doesn't exist at the expected path. Consider: listing the parent directory first, checking the workspace tree, or trying alternative path patterns.",
        "command_not_found" =>
            "The command is not available on this system. Consider: using a different tool (read, search), or checking what shell utilities are available.",
        e if e.starts_with("exit_code_") =>
            "The command exited with a non-zero status code. The tool or path may not be available in this context. Consider checking the command output for specific error details.",
        _ =>
            "The command failed for an unexpected reason. Consider: using a different tool type (read/search instead of shell), or breaking the task into smaller, simpler steps.",
    };

    let scope_guidance = match scope {
        "wide" => "This was a wide-scope operation. Try narrowing with -maxdepth, specific base paths, or file-type filters (-name '*.ext').",
        "medium" => "Try further narrowing the scope, or split into per-subdirectory passes.",
        "narrow" => "Even with narrow scope this failed. The issue may be tool choice rather than scope — consider read or search tools.",
        _ => "",
    };

    format!(
        "Strategy '{}' failed with error '{}'.\n\n{}\n\n{}",
        failed_strategy, error_class, error_guidance, scope_guidance
    )
}

/// Lightweight error classification for repeated-failure detection.
fn classify_error(result: &crate::tool_calling::ToolExecutionResult) -> String {
    if result.timed_out {
        return "timeout".to_string();
    }
    if let Some(sig) = result.signal_killed {
        return format!("killed_signal_{}", sig);
    }
    let lower = result.content.to_ascii_lowercase();
    if lower.contains("permission denied") || lower.contains("access denied") {
        "permission_denied".to_string()
    } else if lower.contains("no such file") || lower.contains("not found") {
        "not_found".to_string()
    } else if lower.contains("command not found") || lower.contains("unknown command") {
        "command_not_found".to_string()
    } else if let Some(ec) = result.exit_code {
        format!("exit_code_{}", ec)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_budget_matches_legacy_constants() {
        let b = StageBudget::default();
        assert_eq!(b.max_iterations, 0); // 0 = unlimited
        assert_eq!(b.max_stagnation_cycles, 8);
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
            exit_code: None,
            timed_out: false,
            status: crate::tools::ToolStatus::Failed,
            duration_ms: 0,
            signal_killed: None,
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

        // Strategy 2 fails (different strategy class, same args)
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
            "Should not stop on first failure of strategy 2"
        );

        // Strategy 2 fails again (same strategy class, same args)
        let call3 = ToolCall {
            id: "c3".to_string(),
            call_type: "function".to_string(),
            function: ToolFunctionCall {
                name: "shell".to_string(),
                arguments: r#"{"command":"du -sh ."}"#.to_string(),
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

    #[test]
    fn test_retry_loop_detection_same_strategy() {
        let budget = StageBudget {
            max_repeated_failures: 10, // high so only retry loop fires
            ..Default::default()
        };
        let mut policy = StopPolicy::new(budget);

        let mut fail_result = crate::tool_calling::ToolExecutionResult {
            tool_call_id: "c1".to_string(),
            tool_name: "shell".to_string(),
            content: "timed out after 20s".to_string(),
            ok: false,
            exit_code: None,
            timed_out: true,
            status: crate::tools::ToolStatus::TimedOut,
            duration_ms: 0,
            signal_killed: None,
        };

        // Same find strategy fails 3 times
        for i in 1..=3 {
            let call = ToolCall {
                id: format!("c{}", i),
                call_type: "function".to_string(),
                function: ToolFunctionCall {
                    name: "shell".to_string(),
                    arguments: r#"{"command":"find . -type f"}"#.to_string(),
                },
            };
            fail_result.tool_call_id = format!("c{}", i);
            policy.record_tool_result(&call, &fail_result);
        }

        assert!(
            policy.is_retry_loop_detected(),
            "Should detect retry loop after 3 consecutive same-strategy failures"
        );
        assert_eq!(
            policy.consecutive_shell_failures(),
            3,
            "Should have 3 consecutive failures"
        );
        assert!(
            policy.strategy_shift_hint().is_some(),
            "Should generate strategy-shift hint"
        );
    }

    #[test]
    fn test_retry_loop_resets_on_strategy_change() {
        let budget = StageBudget {
            max_repeated_failures: 10,
            ..Default::default()
        };
        let mut policy = StopPolicy::new(budget);

        let mut fail_result = crate::tool_calling::ToolExecutionResult {
            tool_call_id: "c1".to_string(),
            tool_name: "shell".to_string(),
            content: "timed out".to_string(),
            ok: false,
            exit_code: None,
            timed_out: true,
            status: crate::tools::ToolStatus::TimedOut,
            duration_ms: 0,
            signal_killed: None,
        };

        // find_other fails twice
        for i in 1..=2 {
            let call = ToolCall {
                id: format!("c{}", i),
                call_type: "function".to_string(),
                function: ToolFunctionCall {
                    name: "shell".to_string(),
                    arguments: r#"{"command":"find . -type f"}"#.to_string(),
                },
            };
            fail_result.tool_call_id = format!("c{}", i);
            policy.record_tool_result(&call, &fail_result);
        }

        // Switch to du_aggregate — should reset counter
        let call3 = ToolCall {
            id: "c3".to_string(),
            call_type: "function".to_string(),
            function: ToolFunctionCall {
                name: "shell".to_string(),
                arguments: r#"{"command":"du -sh ."}"#.to_string(),
            },
        };
        fail_result.tool_call_id = "c3".to_string();
        policy.record_tool_result(&call3, &fail_result);

        assert_eq!(
            policy.consecutive_shell_failures(),
            1,
            "Should reset to 1 after strategy change"
        );
        assert!(
            !policy.is_retry_loop_detected(),
            "Should not detect retry loop after strategy change"
        );
    }

    #[test]
    fn test_retry_loop_resets_on_success() {
        let budget = StageBudget::default();
        let mut policy = StopPolicy::new(budget);

        let mut fail_result = crate::tool_calling::ToolExecutionResult {
            tool_call_id: "c1".to_string(),
            tool_name: "shell".to_string(),
            content: "timed out".to_string(),
            ok: false,
            exit_code: None,
            timed_out: true,
            status: crate::tools::ToolStatus::TimedOut,
            duration_ms: 0,
            signal_killed: None,
        };

        // Fail twice
        for i in 1..=2 {
            let call = ToolCall {
                id: format!("c{}", i),
                call_type: "function".to_string(),
                function: ToolFunctionCall {
                    name: "shell".to_string(),
                    arguments: r#"{"command":"find . -type f"}"#.to_string(),
                },
            };
            fail_result.tool_call_id = format!("c{}", i);
            policy.record_tool_result(&call, &fail_result);
        }

        // Success resets
        let success_call = ToolCall {
            id: "c3".to_string(),
            call_type: "function".to_string(),
            function: ToolFunctionCall {
                name: "shell".to_string(),
                arguments: r#"{"command":"find . -maxdepth 1 -type f"}"#.to_string(),
            },
        };
        let success_result = crate::tool_calling::ToolExecutionResult {
            tool_call_id: "c3".to_string(),
            tool_name: "shell".to_string(),
            content: "file1\nfile2".to_string(),
            ok: true,
            exit_code: None,
            timed_out: false,
            status: crate::tools::ToolStatus::Failed,
            duration_ms: 0,
            signal_killed: None,
        };
        policy.record_tool_result(&success_call, &success_result);

        assert_eq!(
            policy.consecutive_shell_failures(),
            0,
            "Should reset to 0 on success"
        );
        assert!(
            !policy.is_retry_loop_detected(),
            "Should not detect retry loop after success"
        );
    }

    #[test]
    fn test_estimate_command_scope() {
        // Narrow: has maxdepth
        assert_eq!(
            estimate_command_scope("find . -maxdepth 1 -type f"),
            "narrow"
        );
        // Narrow: has head
        assert_eq!(
            estimate_command_scope("find . -type f | head -20"),
            "narrow"
        );
        // Wide: no exclusions, no name filter
        assert_eq!(estimate_command_scope("find . -type f"), "wide");
        // Medium: has name filter but no maxdepth
        assert_eq!(estimate_command_scope("find . -name '*.rs'"), "medium");
    }

    #[test]
    fn test_suggest_alternatives() {
        // timeout error with wide scope should suggest narrowing
        let hint = suggest_alternatives("find_other", "timeout", "wide");
        assert!(hint.contains("narrowing") || hint.contains("maxdepth"));

        // permission denied should suggest accessible dirs
        let hint2 = suggest_alternatives("stat_loop", "permission_denied", "narrow");
        assert!(hint2.contains("permission"));

        // unknown error should have generic recovery guidance
        let hint3 = suggest_alternatives("unknown_strategy", "other", "medium");
        assert!(hint3.contains("read") || hint3.contains("search"));
    }

    // ── T333: Respond abuse guard tests ──

    #[test]
    fn respond_counter_increments() {
        let budget = StageBudget::default();
        let mut policy = StopPolicy::new(budget);
        assert_eq!(policy.consecutive_respond_calls(), 0);
        policy.increment_respond_counter();
        assert_eq!(policy.consecutive_respond_calls(), 1);
        policy.increment_respond_counter();
        policy.increment_respond_counter();
        assert_eq!(policy.consecutive_respond_calls(), 3);
    }

    #[test]
    fn respond_counter_resets_on_real_tool() {
        let budget = StageBudget::default();
        let mut policy = StopPolicy::new(budget);
        policy.increment_respond_counter();
        policy.increment_respond_counter();
        assert_eq!(policy.consecutive_respond_calls(), 2);
        policy.reset_respond_counter();
        assert_eq!(policy.consecutive_respond_calls(), 0);
        assert_eq!(policy.consecutive_respond_only_turns, 0);
    }

    #[test]
    fn mark_real_tool_call_sets_flag() {
        let budget = StageBudget::default();
        let mut policy = StopPolicy::new(budget);
        assert!(!policy.has_real_tool_calls_this_turn());
        policy.mark_real_tool_call();
        assert!(policy.has_real_tool_calls_this_turn());
    }

    #[test]
    fn start_iteration_resets_real_tool_flag() {
        let budget = StageBudget::default();
        let mut policy = StopPolicy::new(budget);
        policy.mark_real_tool_call();
        assert!(policy.has_real_tool_calls_this_turn());
        policy.start_iteration();
        assert!(!policy.has_real_tool_calls_this_turn());
    }

    #[test]
    fn respond_only_turns_stops_at_5() {
        let budget = StageBudget::default();
        let mut policy = StopPolicy::new(budget);
        assert!(policy.record_respond_only_turn().is_none());
        assert!(policy.record_respond_only_turn().is_none());
        assert!(policy.record_respond_only_turn().is_none());
        assert!(policy.record_respond_only_turn().is_none());
        let outcome = policy.record_respond_only_turn();
        assert!(outcome.is_some());
        assert_eq!(outcome.unwrap().reason, StopReason::RespondAbuse);
    }

    #[test]
    fn respond_only_turns_resets_with_counter() {
        let budget = StageBudget::default();
        let mut policy = StopPolicy::new(budget);
        policy.record_respond_only_turn();
        policy.record_respond_only_turn();
        assert_eq!(policy.consecutive_respond_only_turns, 2);
        policy.reset_respond_counter();
        assert_eq!(policy.consecutive_respond_only_turns, 0);
    }

    #[test]
    fn test_is_struggling_detection() {
        let mut budget = StageBudget::default();
        budget.max_repeated_failures = 2;
        budget.max_stagnation_cycles = 2;
        let mut policy = StopPolicy::new(budget.clone());

        // Initially not struggling
        assert!(!policy.is_struggling());

        // Add repeated failures
        policy
            .tool_failures
            .push(("cmd".to_string(), "error".to_string()));
        policy
            .tool_failures
            .push(("cmd".to_string(), "error".to_string()));
        assert!(policy.is_struggling());

        // Reset and test stagnation
        let mut policy2 = StopPolicy::new(budget.clone());
        policy2.stagnation_runs = 2;
        assert!(policy2.is_struggling());

        // Test retry loop
        let mut policy3 = StopPolicy::new(budget);
        policy3.retry_loop_detected = true;
        assert!(policy3.is_struggling());
    }

    #[test]
    fn respond_abuse_reason_has_hint() {
        let hint = next_step_hint(&StopReason::RespondAbuse);
        assert!(hint.contains("search") || hint.contains("read") || hint.contains("shell"));
    }

    // ── Regression tests ──

    #[test]
    fn regression_chat_loop_should_trigger_respond_abuse() {
        let budget = StageBudget { max_stagnation_cycles: 10, ..Default::default() };
        let mut policy = StopPolicy::new(budget);
        // Simulate 5 respond-only turns
        for _ in 0..4 {
            assert!(policy.record_respond_only_turn().is_none());
        }
        let outcome = policy.record_respond_only_turn();
        assert!(outcome.is_some());
        assert_eq!(outcome.unwrap().reason, StopReason::RespondAbuse);
    }

    #[test]
    fn regression_read_read_respond_workflow_does_not_stop() {
        let budget = StageBudget { max_stagnation_cycles: 8, ..Default::default() };
        let mut policy = StopPolicy::new(budget);

        // Simulate: 3 normal tool calls (read, read, respond) — should not stop
        let calls = vec![
            ToolCall { id: "c1".to_string(), call_type: "function".to_string(), function: ToolFunctionCall { name: "read".to_string(), arguments: r#"{"path":"a"}"#.to_string() } },
            ToolCall { id: "c2".to_string(), call_type: "function".to_string(), function: ToolFunctionCall { name: "read".to_string(), arguments: r#"{"path":"b"}"#.to_string() } },
            ToolCall { id: "c3".to_string(), call_type: "function".to_string(), function: ToolFunctionCall { name: "respond".to_string(), arguments: r#"{"content":"done"}"#.to_string() } },
        ];

        // Each call registers as a new signal
        policy.start_iteration();
        assert!(policy.record_tool_calls(&calls[..1]).is_none());
        policy.register_signal("read:a".to_string());
        policy.record_new_signals();

        policy.start_iteration();
        assert!(policy.record_tool_calls(&calls[1..2]).is_none());
        policy.register_signal("read:b".to_string());
        policy.record_new_signals();

        policy.start_iteration();
        assert!(policy.record_tool_calls(&calls[2..]).is_none());
        policy.register_signal("respond:done".to_string());
        policy.record_new_signals();

        assert!(policy.check_should_stop().is_none());
    }

    #[test]
    fn regression_same_tool_different_commands_should_not_stagnate() {
        let budget = StageBudget::default();
        let mut policy = StopPolicy::new(budget);

        policy.start_iteration();
        assert!(policy.register_signal("shell:find . -name '*.rs'".to_string()));
        policy.record_new_signals();

        policy.start_iteration();
        assert!(policy.register_signal("shell:find . -name '*.py'".to_string()));
        policy.record_new_signals();

        policy.start_iteration();
        assert!(policy.register_signal("shell:find . -maxdepth 1".to_string()));
        policy.record_new_signals();

        // Changing arguments should not trigger stagnation
        assert!(!policy.is_struggling());
    }

    #[test]
    fn regression_exact_repeat_commands_should_stagnate() {
        let budget = StageBudget { max_stagnation_cycles: 3, ..Default::default() };
        let mut policy = StopPolicy::new(budget);

        // Same command repeated
        let cmd = "find . -type f";
        policy.start_iteration();
        assert!(policy.register_signal(cmd.to_string()));
        policy.record_new_signals();

        policy.start_iteration();
        assert!(!policy.register_signal(cmd.to_string())); // already seen
        assert!(policy.record_stagnation().is_none()); // run 1, under threshold

        policy.start_iteration();
        assert!(!policy.register_signal(cmd.to_string()));
        assert!(policy.record_stagnation().is_none()); // run 2, under threshold

        policy.start_iteration();
        // Third stagnation should trigger
        let outcome = policy.record_stagnation();
        assert!(outcome.is_some(), "3 stagnation runs should trigger stop");
    }

    #[test]
    fn regression_goal_consistency_fires_at_18_tool_calls() {
        let budget = StageBudget::default();
        let mut policy = StopPolicy::new(budget);

        assert!(!policy.goal_consistency_check_needed());

        // 17 calls should not fire
        for _ in 0..17 {
            policy.total_tool_calls += 1;
        }
        assert!(!policy.goal_consistency_check_needed());

        // 18th call should fire
        policy.total_tool_calls += 1;
        assert!(policy.goal_consistency_check_needed());

        // Next check should not fire again at same milestone
        assert!(!policy.goal_consistency_check_needed());
    }

    #[test]
    fn regression_wall_clock_budget_stops_after_timeout() {
        let budget = StageBudget { max_wall_clock_s: 1, ..Default::default() };
        let mut policy = StopPolicy::new(budget);
        // Set start_time to a past time to simulate wall clock expiry
        policy.start_time = std::time::Instant::now() - std::time::Duration::from_secs(2);
        let outcome = policy.start_iteration();
        assert!(outcome.is_some(), "wall clock budget should trigger stop when exceeded");
        assert_eq!(outcome.unwrap().reason.as_str(), "wall_clock_exceeded");
    }
}
