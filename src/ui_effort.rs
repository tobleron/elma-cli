//! @efficiency-role: ui-component
//!
//! Task 107: Visual Effort Indicator
//!
//! Measures and displays wall-clock time per turn.
//! Shows a subtle effort badge after each response.

use crate::ui_theme::*;
use std::time::Instant;

/// Simple wall-clock timer for measuring turn effort.
pub(crate) struct EffortTimer {
    start: Instant,
}

impl EffortTimer {
    /// Start measuring effort time.
    pub(crate) fn start() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    /// Get elapsed duration.
    pub(crate) fn elapsed(&self) -> std::time::Duration {
        self.start.elapsed()
    }

    /// Format the elapsed time as a user-visible effort badge.
    pub(crate) fn format(&self) -> String {
        format_effort_indicator(self.elapsed())
    }
}

/// Format a duration as a human-readable effort indicator.
pub(crate) fn format_effort_indicator(duration: std::time::Duration) -> String {
    let secs = duration.as_secs();
    let millis = duration.subsec_millis();

    if secs < 1 {
        format!("⏱ {}ms", millis)
    } else if secs < 60 {
        format!("⏱ {}.{}s", secs, millis / 100)
    } else {
        let mins = secs / 60;
        let remaining_secs = secs % 60;
        format!("⏱ {}m {}s", mins, remaining_secs)
    }
}

/// Color-code the effort indicator based on duration.
/// Fast = green, moderate = yellow, slow = red
pub(crate) fn format_effort_colored(duration: std::time::Duration) -> String {
    let text = format_effort_indicator(duration);
    let secs = duration.as_secs_f64();

    if secs < 1.0 {
        success_green(&text)
    } else if secs < 5.0 {
        warn_yellow(&text)
    } else {
        error_red(&text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effort_format_milliseconds() {
        let d = std::time::Duration::from_millis(350);
        assert_eq!(format_effort_indicator(d), "⏱ 350ms");
    }

    #[test]
    fn test_effort_format_seconds() {
        let d = std::time::Duration::from_millis(2350);
        assert_eq!(format_effort_indicator(d), "⏱ 2.3s");
    }

    #[test]
    fn test_effort_format_minutes() {
        let d = std::time::Duration::from_secs(83);
        assert_eq!(format_effort_indicator(d), "⏱ 1m 23s");
    }

    #[test]
    fn test_effort_timer_basic() {
        let timer = EffortTimer::start();
        std::thread::sleep(std::time::Duration::from_millis(50));
        let elapsed = timer.elapsed();
        assert!(elapsed.as_millis() >= 50);
        assert!(!timer.format().is_empty());
    }

    #[test]
    fn test_effort_colored_output() {
        let fast = format_effort_colored(std::time::Duration::from_millis(500));
        assert!(fast.contains("⏱"));
        let slow = format_effort_colored(std::time::Duration::from_secs(10));
        assert!(slow.contains("⏱"));
    }
}
