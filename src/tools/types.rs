//! @efficiency-role: data-model
//! Tool execution result type and related data structures.

/// Structured status for tool execution results.
/// Replaces the binary ok/fail pattern with fine-grained classification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolStatus {
    Success,
    SuccessEmpty,
    Failed,
    ValidationFailed,
    Blocked,
    TimedOut,
    ToolNotFound,
    ExecutionError,
}

impl ToolStatus {
    pub fn is_ok(&self) -> bool {
        matches!(self, ToolStatus::Success | ToolStatus::SuccessEmpty)
    }

    pub fn is_error(&self) -> bool {
        !self.is_ok()
    }
}

#[derive(Debug, Clone)]
pub struct ToolExecutionResult {
    pub tool_call_id: String,
    pub tool_name: String,
    pub content: String,
    /// True if the tool completed successfully (backward compat).
    pub ok: bool,
    /// Structured status for fine-grained classification.
    pub status: ToolStatus,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub signal_killed: Option<i32>,
    /// Execution wall-clock time in milliseconds.
    pub duration_ms: u64,
}

impl ToolExecutionResult {
    pub fn new_ok(call_id: &str, tool_name: &str, content: &str) -> Self {
        Self {
            tool_call_id: call_id.to_string(),
            tool_name: tool_name.to_string(),
            content: content.to_string(),
            ok: true,
            status: ToolStatus::Success,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
            duration_ms: 0,
        }
    }

    pub fn new_failed(call_id: &str, tool_name: &str, content: &str) -> Self {
        Self {
            tool_call_id: call_id.to_string(),
            tool_name: tool_name.to_string(),
            content: content.to_string(),
            ok: false,
            status: ToolStatus::Failed,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
            duration_ms: 0,
        }
    }

    pub fn new_blocked(call_id: &str, tool_name: &str, content: &str) -> Self {
        Self {
            tool_call_id: call_id.to_string(),
            tool_name: tool_name.to_string(),
            content: content.to_string(),
            ok: false,
            status: ToolStatus::Blocked,
            exit_code: None,
            timed_out: false,
            signal_killed: None,
            duration_ms: 0,
        }
    }

    pub fn from_shell(
        call_id: &str,
        tool_name: &str,
        content: &str,
        exit_code: i32,
        timed_out: bool,
        duration_ms: u64,
    ) -> Self {
        let status = if timed_out { ToolStatus::TimedOut }
            else if exit_code == 0 { ToolStatus::Success }
            else { ToolStatus::ExecutionError };
        Self {
            tool_call_id: call_id.to_string(),
            tool_name: tool_name.to_string(),
            content: content.to_string(),
            ok: exit_code == 0,
            status,
            exit_code: Some(exit_code),
            timed_out,
            signal_killed: None,
            duration_ms,
        }
    }
}
