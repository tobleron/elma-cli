//! @efficiency-role: ui-component
//!
//! Claude Code-style Session Lifecycle
//!
//! Session management:
//! - Clear (/clear)
//! - Resume (/resume)
//! - Exit (/exit, Ctrl+D)
//! - Double-press detection

use crate::ui_theme::*;

// ============================================================================
// Exit State
// ============================================================================

#[derive(Clone, Debug, Default)]
pub(crate) struct ExitState {
    pub requested: bool,
    pub double_press: bool,
    pub countdown_ms: u64,
}

impl ExitState {
    pub(crate) fn new() -> Self {
        Self {
            requested: false,
            double_press: false,
            countdown_ms: 1000, // 1 second window
        }
    }

    pub(crate) fn request_exit(&mut self) -> bool {
        if self.requested && !self.double_press {
            self.double_press = true;
            return true;
        }
        self.requested = true;
        false
    }

    pub(crate) fn reset(&mut self) {
        self.requested = false;
        self.double_press = false;
    }

    pub(crate) fn render_prompt(&self, base_prompt: &str) -> String {
        if self.double_press {
            dim("Press again to confirm exit")
        } else {
            base_prompt.to_string()
        }
    }
}

// ============================================================================
// Session Commands
// ============================================================================

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum SessionCommand {
    None,
    Clear,
    Resume,
    Exit,
    History,
}

impl SessionCommand {
    pub(crate) fn parse(input: &str) -> Self {
        let trimmed = input.trim();
        match trimmed {
            "/clear" => SessionCommand::Clear,
            "/resume" => SessionCommand::Resume,
            "/exit" | "/quit" => SessionCommand::Exit,
            "/history" | "/hist" => SessionCommand::History,
            _ => SessionCommand::None,
        }
    }
}

// ============================================================================
// Session State
// ============================================================================

#[derive(Clone, Debug, Default)]
pub(crate) struct SessionState {
    pub id: Option<String>,
    pub resumed: bool,
    pub cleared: bool,
    pub message_count: usize,
}

impl SessionState {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn message_count(&self) -> usize {
        self.message_count
    }

    pub(crate) fn increment_messages(&mut self) {
        self.message_count += 1;
    }

    pub(crate) fn clear(&mut self) {
        self.cleared = true;
        self.message_count = 0;
    }

    pub(crate) fn resume(&mut self, session_id: String) {
        self.id = Some(session_id);
        self.resumed = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_command_parse() {
        assert_eq!(SessionCommand::parse("/clear"), SessionCommand::Clear);
        assert_eq!(SessionCommand::parse("/exit"), SessionCommand::Exit);
        assert_eq!(SessionCommand::parse("hello"), SessionCommand::None);
    }

    #[test]
    fn test_exit_double_press() {
        let mut e = ExitState::new();
        assert!(!e.request_exit());
        assert!(e.request_exit());
    }

    #[test]
    fn test_session_message_count() {
        let mut s = SessionState::new();
        assert_eq!(s.message_count(), 0);
        s.increment_messages();
        assert_eq!(s.message_count(), 1);
    }
}
