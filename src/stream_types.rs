//! @efficiency-role: data-model
//! Shared types for SSE streaming and chat event parsing.

/// A single SSE frame from the streaming response.
#[derive(Debug, Clone)]
pub struct SseFrame {
    pub event: Option<String>,
    pub data: String,
}

/// Typed event emitted during chat completion streaming.
#[derive(Debug, Clone)]
pub enum ChatStreamEvent {
    ContentDelta(String),
    ReasoningDelta(String),
    ToolCallDelta(ToolCallDelta),
    ThinkingStarted,
    ThinkingFinished,
    ContentFinished,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct ToolCallDelta {
    pub index: usize,
    pub id: Option<String>,
    pub name: Option<String>,
    pub arguments: String,
}

/// Accumulated state during tool call streaming.
#[derive(Default)]
pub struct StreamingToolCallPart {
    pub id: Option<String>,
    pub call_type: Option<String>,
    pub name: Option<String>,
    pub arguments: String,
}
