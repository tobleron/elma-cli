//! @efficiency-role: ui-component
//!
//! Claude Code-style Terminal UI Renderer
//!
//! Design (from Claude Code study):
//! - Sparse message rows, no persistent header/activity/context chrome
//! - User: "> " prefix
//! - Assistant: "● " prefix with markdown
//! - Thinking: "∴ Thinking" collapsed, expanded in transcript mode
//! - Tools: "▸ " start, "✓" / "✗" result
//! - Prompt at bottom, transient picker modals only

pub mod claude_input;
pub mod claude_markdown;
pub mod claude_render;
pub mod claude_session;
pub mod claude_state;
pub mod claude_status;
pub mod claude_stream;
pub mod claude_tasks;

pub use claude_input::*;
pub use claude_markdown::*;
pub use claude_render::*;
pub use claude_session::*;
pub use claude_state::*;
pub use claude_status::*;
pub use claude_stream::*;
pub use claude_tasks::*;

// ============================================================================
// UI Event Boundary (Task 169)
// ============================================================================

#[derive(Clone, Debug)]
pub enum UiEvent {
    TurnStarted,
    UserSubmitted(String),
    ThinkingStarted,
    ThinkingDelta(String),
    ThinkingFinished,
    AssistantContentDelta(String),
    AssistantFinished,
    ToolStarted {
        name: String,
        command: String,
    },
    ToolProgress {
        name: String,
        message: String,
    },
    ToolFinished {
        name: String,
        success: bool,
        output: String,
    },
    PermissionRequested {
        command: String,
    },
    PermissionResolved {
        command: String,
        approved: bool,
    },
    TasksUpdated,
    CompactBoundary,
    StatusUpdated {
        model: String,
        ctx_tokens: u64,
    },
    Notification {
        message: String,
        level: String,
    },
    InputChanged(String),
    ModeChanged(String),
    Resize {
        cols: usize,
        rows: usize,
    },
    ExitRequested,
}
