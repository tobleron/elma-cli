//! @efficiency-role: domain-logic
//!
//! Status Thread Module
//!
//! A persistent activity indicator that stays visible at the end of the chat history.
//! Shows a spinner during active work, a checkmark on completion, and enforces a
//! minimum 2-second visibility window so the user can read what happened.
//!
//! Task 311: Persistent Status Thread

use std::time::{Duration, Instant};

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const SPINNER_INTERVAL_MS: u64 = 100;
const MIN_VISIBLE_SECS: u64 = 2;

#[derive(Debug, Clone)]
pub(crate) enum StatusState {
    Idle,
    Working {
        description: String,
        started_at: Instant,
    },
    Completed {
        description: String,
        completed_at: Instant,
    },
}

#[derive(Debug, Clone)]
pub(crate) struct StatusThread {
    state: StatusState,
    spinner_frame: u8,
    last_frame_advance: Instant,
    min_visible_until: Option<Instant>,
}

impl Default for StatusThread {
    fn default() -> Self {
        Self::new()
    }
}

impl StatusThread {
    pub fn new() -> Self {
        Self {
            state: StatusState::Idle,
            spinner_frame: 0,
            last_frame_advance: Instant::now(),
            min_visible_until: None,
        }
    }

    /// Start a new working status with the given description.
    pub fn start(&mut self, description: &str) {
        self.state = StatusState::Working {
            description: description.to_string(),
            started_at: Instant::now(),
        };
        self.min_visible_until = None;
    }

    /// Mark the current status as completed. The status will remain visible
    /// for at least 2 seconds before it can be cleared.
    pub fn complete(&mut self, description: &str) {
        let now = Instant::now();
        self.state = StatusState::Completed {
            description: description.to_string(),
            completed_at: now,
        };
        self.min_visible_until = Some(now + Duration::from_secs(MIN_VISIBLE_SECS));
    }

    /// Clear the status, respecting the minimum visibility window.
    /// If less than 2 seconds have passed since completion, this is a no-op.
    pub fn clear(&mut self) {
        if let Some(until) = &self.min_visible_until {
            if Instant::now() < *until {
                return;
            }
        }
        self.state = StatusState::Idle;
        self.min_visible_until = None;
    }

    /// Get the current rendered line. Returns None if idle or past visibility window.
    /// Advances the spinner frame if working.
    pub fn render(&mut self) -> Option<String> {
        let state_clone = self.state.clone();
        match state_clone {
            StatusState::Idle => None,
            StatusState::Working { .. } => {
                self.advance_spinner();
                let spinner = self.current_spinner_char();
                if let StatusState::Working { description, .. } = &self.state {
                    Some(format!("{} {}", spinner, description))
                } else {
                    None
                }
            }
            StatusState::Completed {
                description,
                completed_at,
            } => {
                let elapsed = completed_at.elapsed();
                if elapsed < Duration::from_secs(MIN_VISIBLE_SECS + 1) {
                    Some(format!("✓ {}", description))
                } else {
                    self.state = StatusState::Idle;
                    self.min_visible_until = None;
                    None
                }
            }
        }
    }

    /// Returns true if the status thread is currently in a working state.
    pub fn is_working(&self) -> bool {
        matches!(self.state, StatusState::Working { .. })
    }

    /// Returns the current description (without spinner/checkmark).
    pub fn description(&self) -> Option<String> {
        match &self.state {
            StatusState::Idle => None,
            StatusState::Working { description, .. }
            | StatusState::Completed { description, .. } => Some(description.clone()),
        }
    }

    fn advance_spinner(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_frame_advance) >= Duration::from_millis(SPINNER_INTERVAL_MS) {
            self.spinner_frame = (self.spinner_frame + 1) % SPINNER_FRAMES.len() as u8;
            self.last_frame_advance = now;
        }
    }

    fn current_spinner_char(&self) -> &'static str {
        SPINNER_FRAMES[self.spinner_frame as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_thread_starts_idle() {
        let st = StatusThread::new();
        assert!(matches!(st.state, StatusState::Idle));
        assert!(!st.is_working());
        assert!(st.description().is_none());
    }

    #[test]
    fn test_status_thread_start_working() {
        let mut st = StatusThread::new();
        st.start("Testing...");
        assert!(st.is_working());
        assert_eq!(st.description(), Some("Testing...".to_string()));
    }

    #[test]
    fn test_status_thread_complete() {
        let mut st = StatusThread::new();
        st.start("Testing...");
        st.complete("Done");
        assert!(!st.is_working());
        assert_eq!(st.description(), Some("Done".to_string()));
    }

    #[test]
    fn test_status_thread_render_working() {
        let mut st = StatusThread::new();
        st.start("Loading...");
        let rendered = st.render();
        assert!(rendered.is_some());
        let s = rendered.unwrap();
        assert!(s.contains("Loading..."));
    }

    #[test]
    fn test_status_thread_render_idle() {
        let mut st = StatusThread::new();
        assert!(st.render().is_none());
    }

    #[test]
    fn test_status_thread_spinner_advances() {
        let mut st = StatusThread::new();
        st.start("Working...");
        let first = st.render().unwrap();
        std::thread::sleep(Duration::from_millis(150));
        let second = st.render().unwrap();
        assert_ne!(first, second);
    }

    #[test]
    fn test_status_thread_min_visibility() {
        let mut st = StatusThread::new();
        st.start("Working...");
        st.complete("Done");
        let rendered = st.render();
        assert!(rendered.is_some());
        let s = rendered.unwrap();
        assert!(s.starts_with("✓"));
        assert!(s.contains("Done"));
    }
}
