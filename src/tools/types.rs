//! @efficiency-role: data-model
//! Tool execution result type and related data structures.

#[derive(Debug, Clone)]
pub struct ToolExecutionResult {
    pub tool_call_id: String,
    pub tool_name: String,
    pub content: String,
    pub ok: bool,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub signal_killed: Option<i32>,
}
