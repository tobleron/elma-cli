//! @efficiency-role: ui-component
//!
//! Task 109: Indicatif Progress Integration
//!
//! Thread-safe, high-resolution spinners and progress bars for:
//! - LLM thinking phases
//! - Tool execution phases
//!
//! Design: Minimal, Tokyo Night colors, graceful fallback to plain text.

use crate::ui_colors::*;
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use std::io::IsTerminal;
use std::sync::OnceLock;

/// Global MultiProgress manager.
static MULTI_PROGRESS: OnceLock<MultiProgress> = OnceLock::new();

fn get_multi_progress() -> &'static MultiProgress {
    MULTI_PROGRESS.get_or_init(|| {
        MultiProgress::with_draw_target(ProgressDrawTarget::stderr())
    })
}

/// Create a spinner for a single operation.
/// Returns None if not a terminal (graceful fallback).
pub(crate) fn create_spinner(prefix: &str, message: &str) -> Option<ProgressBar> {
    if !std::io::stderr().is_terminal() {
        eprintln!("  {} {}...", info_cyan(prefix), message);
        return None;
    }

    let mp = get_multi_progress();
    let pb = mp.add(ProgressBar::new_spinner());

    // Tokyo Night themed Braille spinner
    pb.set_style(
        ProgressStyle::with_template("{spinner} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb.set_message(format!("{} {}", info_cyan(prefix), message));
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    Some(pb)
}

/// Create a progress bar for multi-step operations.
pub(crate) fn create_progress_bar(prefix: &str, total: u64) -> Option<ProgressBar> {
    if !std::io::stderr().is_terminal() {
        eprintln!("  {} starting ({}/{} steps)...", info_cyan(prefix), 0, total);
        return None;
    }

    let mp = get_multi_progress();
    let pb = mp.add(ProgressBar::new(total));

    pb.set_style(
        ProgressStyle::with_template("[{bar:40}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("█▓▒░"),
    );
    pb.set_message(info_cyan(&format!("{}:", prefix)));

    Some(pb)
}

/// Finish a spinner/progress bar with a status message.
pub(crate) fn finish_progress(pb: &ProgressBar, message: &str, success: bool) {
    let icon = if success { "✓" } else { "✗" };
    let colored = if success {
        format!("{} {}", success_green(icon), message)
    } else {
        format!("{} {}", error_red(icon), message)
    };
    pb.finish_with_message(colored);
}

/// Clear a finished progress bar from display.
pub(crate) fn clear_progress(pb: &ProgressBar) {
    pb.set_draw_target(ProgressDrawTarget::hidden());
}

/// Simple spinner lifecycle helper.
pub(crate) async fn with_spinner<T, F, Fut>(prefix: &str, message: &str, f: F) -> T
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = T>,
{
    let pb = create_spinner(prefix, message);
    let result = f().await;
    if let Some(ref bar) = pb {
        finish_progress(bar, message, true);
        clear_progress(bar);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spinner_creates_in_terminal() {
        let _ = create_spinner("test", "working");
    }

    #[test]
    fn test_progress_bar_creation() {
        let _ = create_progress_bar("steps", 10);
    }

    #[tokio::test]
    async fn test_with_spinner_lifecycle() {
        let result = with_spinner("test", "async work", || async {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            42
        })
        .await;
        assert_eq!(result, 42);
    }
}
