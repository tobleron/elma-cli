//! @efficiency-role: ui-component
//!
//! Task 101: Verb-Driven Loading Spinners
//!
//! Animated Braille spinner with context-aware verbs.
//! Runs in a background std::thread (works in both sync and async contexts).
//! When the TUI is active, spinner output is suppressed to avoid competing with the main UI.

use crate::ui_state::is_tui_active;
use crate::ui_theme::*;
use std::io::{IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const SPINNER_INTERVAL_MS: u64 = 100;

#[derive(Debug, Clone, Copy)]
pub(crate) enum SpinnerVerb {
    Thinking,
    Searching,
    Reading,
    Executing,
    Analyzing,
    Processing,
}

impl SpinnerVerb {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            SpinnerVerb::Thinking => "Thinking",
            SpinnerVerb::Searching => "Searching",
            SpinnerVerb::Reading => "Reading",
            SpinnerVerb::Executing => "Executing",
            SpinnerVerb::Analyzing => "Analyzing",
            SpinnerVerb::Processing => "Processing",
        }
    }
}

fn should_show_spinner() -> bool {
    // Suppress spinner when TUI is active (interactive path has its own status rendering)
    if is_tui_active() {
        return false;
    }
    std::io::stderr().is_terminal()
}

/// An animated spinner that runs in a background std::thread.
pub(crate) struct Spinner {
    cancel: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
    start: Instant,
    verb: SpinnerVerb,
    label: String,
}

impl Spinner {
    /// Start an animated spinner. Writes to stderr, updates in-place.
    pub(crate) fn start(verb: SpinnerVerb, label: &str) -> Self {
        let start = Instant::now();
        let label = label.to_string();

        if !should_show_spinner() {
            eprintln!("  {} {}...", info_cyan(verb.as_str()), label);
            return Self {
                cancel: Arc::new(AtomicBool::new(true)),
                handle: None,
                start,
                verb,
                label,
            };
        }

        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_clone = cancel.clone();
        let label_clone = label.clone();

        let handle = thread::spawn(move || {
            let mut frame = 0;
            while !cancel_clone.load(Ordering::Relaxed) {
                let elapsed = start.elapsed().as_secs_f32();
                eprint!(
                    "\r  {} {} {} ({:.1}s)...",
                    SPINNER_FRAMES[frame % SPINNER_FRAMES.len()],
                    info_cyan(verb.as_str()),
                    label_clone,
                    elapsed
                );
                let _ = std::io::stderr().flush();
                thread::sleep(std::time::Duration::from_millis(SPINNER_INTERVAL_MS));
                frame += 1;
            }
        });

        Self {
            cancel,
            handle: Some(handle),
            start,
            verb,
            label,
        }
    }

    /// Stop the spinner and print a completion line.
    pub(crate) fn finish(self, success: bool) -> std::time::Duration {
        self.cancel.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle {
            thread::sleep(std::time::Duration::from_millis(20));
            drop(h);
        }

        let elapsed = self.start.elapsed();
        let (icon, color_fn) = if success {
            ("✓", success_green as fn(&str) -> String)
        } else {
            ("✗", error_red as fn(&str) -> String)
        };

        if should_show_spinner() {
            eprintln!(
                "\r  {} {} {} completed ({:.1}s)",
                color_fn(icon),
                info_cyan(self.verb.as_str()),
                self.label,
                elapsed.as_secs_f32()
            );
            let _ = std::io::stderr().flush();
        } else {
            eprintln!(
                "  {} {} completed ({:.1}s)",
                color_fn(icon),
                self.verb.as_str(),
                elapsed.as_secs_f32()
            );
        }

        elapsed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spinner_verb_display() {
        assert_eq!(SpinnerVerb::Thinking.as_str(), "Thinking");
        assert_eq!(SpinnerVerb::Executing.as_str(), "Executing");
    }

    #[test]
    fn test_spinner_frames() {
        assert_eq!(SPINNER_FRAMES.len(), 10);
    }

    #[test]
    fn test_spinner_start_finish() {
        let spinner = Spinner::start(SpinnerVerb::Executing, "test command");
        thread::sleep(std::time::Duration::from_millis(150));
        let elapsed = spinner.finish(true);
        assert!(elapsed.as_millis() >= 100);
    }

    #[test]
    fn test_spinner_quick_finish() {
        let spinner = Spinner::start(SpinnerVerb::Thinking, "quick");
        let elapsed = spinner.finish(true);
        assert!(elapsed.as_micros() >= 0);
    }
}
