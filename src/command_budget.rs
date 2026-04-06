//! @efficiency-role: domain-logic
//!
//! Command Budget & Rate Limiting (Task 121)
//!
//! Tracks per-session command budgets to prevent runaway loops:
//! - Safe commands: unlimited
//! - Caution commands: max 20/session
//! - Dangerous commands: max 5/session
//! - Rate limit: max 10 shell calls per turn, 100ms throttle

use crate::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

/// Maximum caution-level commands per session.
const MAX_CAUTION_PER_SESSION: usize = 20;
/// Maximum dangerous-level commands per session.
const MAX_DANGER_PER_SESSION: usize = 5;
/// Maximum shell tool calls per single turn.
const MAX_SHELL_CALLS_PER_TURN: usize = 10;
/// Minimum time between commands (prevents runaway loops).
const COMMAND_THROTTLE_MS: u64 = 100;

/// Per-session command budget tracker.
pub(crate) struct CommandBudget {
    /// Number of caution-level commands executed this session.
    caution_count: AtomicUsize,
    /// Number of dangerous-level commands executed this session.
    danger_count: AtomicUsize,
    /// Number of shell tool calls in the current turn.
    turn_shell_calls: AtomicUsize,
    /// Timestamp of the last command execution (for throttling).
    last_command_time: std::sync::Mutex<Option<Instant>>,
}

impl CommandBudget {
    pub(crate) fn new() -> Self {
        Self {
            caution_count: AtomicUsize::new(0),
            danger_count: AtomicUsize::new(0),
            turn_shell_calls: AtomicUsize::new(0),
            last_command_time: std::sync::Mutex::new(None),
        }
    }

    /// Reset all budget counters (called on /reset or new session).
    pub(crate) fn reset(&self) {
        self.caution_count.store(0, Ordering::Relaxed);
        self.danger_count.store(0, Ordering::Relaxed);
        self.turn_shell_calls.store(0, Ordering::Relaxed);
        *self.last_command_time.lock().unwrap() = None;
    }

    /// Start a new turn (reset per-turn shell call counter).
    pub(crate) fn start_turn(&self) {
        self.turn_shell_calls.store(0, Ordering::Relaxed);
    }

    /// Check if a command is within budget.
    /// Returns Ok(()) if allowed, or Err(message) if blocked.
    pub(crate) fn check_budget(&self, risk: &shell_preflight::RiskLevel) -> Result<(), String> {
        // Throttle check (skip if no previous command recorded)
        {
            let mut last_time = self.last_command_time.lock().unwrap();
            if let Some(last) = *last_time {
                let elapsed = last.elapsed().as_millis() as u64;
                if elapsed < COMMAND_THROTTLE_MS {
                    return Err(format!(
                        "Command rate limited: {}ms since last command (minimum {}ms). Slow down.",
                        elapsed, COMMAND_THROTTLE_MS
                    ));
                }
            }
            // Only update time if we're actually going to allow the command
            // (do this in record_command instead to avoid double-updates)
        }

        // Per-turn shell call limit
        let current_turn_calls = self.turn_shell_calls.load(Ordering::Relaxed);
        if current_turn_calls >= MAX_SHELL_CALLS_PER_TURN {
            return Err(format!(
                "Shell call budget exhausted: {}/{} calls this turn. The model is making too many shell calls in a single turn.",
                current_turn_calls, MAX_SHELL_CALLS_PER_TURN
            ));
        }

        match risk {
            shell_preflight::RiskLevel::Safe => {
                // Safe commands: unlimited, but still count turn calls
                Ok(())
            }
            shell_preflight::RiskLevel::Caution => {
                let count = self.caution_count.load(Ordering::Relaxed);
                if count >= MAX_CAUTION_PER_SESSION {
                    return Err(format!(
                        "Caution command budget exhausted: {}/{} used this session. \
                        Use safer alternatives or reset the session.",
                        count, MAX_CAUTION_PER_SESSION
                    ));
                }
                Ok(())
            }
            shell_preflight::RiskLevel::Dangerous(reason) => {
                // Dangerous commands are already blocked by preflight, but track if they get through
                let count = self.danger_count.load(Ordering::Relaxed);
                if count >= MAX_DANGER_PER_SESSION {
                    return Err(format!(
                        "Dangerous command budget exhausted: {}/{} used this session. \
                        Reason: {}. Session must be reset.",
                        count, MAX_DANGER_PER_SESSION, reason
                    ));
                }
                Ok(())
            }
        }
    }

    /// Record that a command was executed.
    pub(crate) fn record_command(&self, risk: &shell_preflight::RiskLevel) {
        // Update throttle timestamp
        *self.last_command_time.lock().unwrap() = Some(Instant::now());

        match risk {
            shell_preflight::RiskLevel::Safe => {
                self.turn_shell_calls.fetch_add(1, Ordering::Relaxed);
            }
            shell_preflight::RiskLevel::Caution => {
                self.caution_count.fetch_add(1, Ordering::Relaxed);
                self.turn_shell_calls.fetch_add(1, Ordering::Relaxed);
            }
            shell_preflight::RiskLevel::Dangerous(_) => {
                self.danger_count.fetch_add(1, Ordering::Relaxed);
                self.turn_shell_calls.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// Get current budget status for tracing.
    pub(crate) fn status(&self) -> String {
        format!(
            "caution: {}/{}, danger: {}/{}, turn_calls: {}/{}",
            self.caution_count.load(Ordering::Relaxed),
            MAX_CAUTION_PER_SESSION,
            self.danger_count.load(Ordering::Relaxed),
            MAX_DANGER_PER_SESSION,
            self.turn_shell_calls.load(Ordering::Relaxed),
            MAX_SHELL_CALLS_PER_TURN
        )
    }
}

/// Global command budget (one per runtime session).
static SESSION_BUDGET: OnceLock<CommandBudget> = OnceLock::new();

/// Get or create the session command budget.
pub(crate) fn get_budget() -> &'static CommandBudget {
    SESSION_BUDGET.get_or_init(|| CommandBudget::new())
}

/// Reset the session command budget (called on /reset).
pub(crate) fn reset_budget() {
    get_budget().reset();
}

#[cfg(test)]
mod tests {
    use super::*;
    use shell_preflight::RiskLevel;

    #[test]
    fn test_safe_commands_unlimited() {
        let budget = CommandBudget::new();
        // Execute 100 safe commands — should never block
        for _ in 0..100 {
            budget.start_turn();
            *budget.last_command_time.lock().unwrap() = None; // Reset throttle
            assert!(budget.check_budget(&RiskLevel::Safe).is_ok());
            budget.record_command(&RiskLevel::Safe);
        }
    }

    #[test]
    fn test_caution_budget_enforced() {
        let budget = CommandBudget::new();
        for i in 0..MAX_CAUTION_PER_SESSION {
            budget.start_turn();
            *budget.last_command_time.lock().unwrap() = None; // Reset throttle
            assert!(
                budget.check_budget(&RiskLevel::Caution).is_ok(),
                "Failed at iteration {}",
                i
            );
            budget.record_command(&RiskLevel::Caution);
        }
        // Next one should fail
        budget.start_turn();
        *budget.last_command_time.lock().unwrap() = None;
        assert!(budget.check_budget(&RiskLevel::Caution).is_err());
    }

    #[test]
    fn test_danger_budget_enforced() {
        let budget = CommandBudget::new();
        let danger = RiskLevel::Dangerous("test".to_string());
        for i in 0..MAX_DANGER_PER_SESSION {
            budget.start_turn();
            *budget.last_command_time.lock().unwrap() = None; // Reset throttle
            assert!(
                budget.check_budget(&danger).is_ok(),
                "Failed at iteration {}",
                i
            );
            budget.record_command(&danger);
        }
        // Next one should fail
        budget.start_turn();
        *budget.last_command_time.lock().unwrap() = None;
        assert!(budget.check_budget(&danger).is_err());
    }

    #[test]
    fn test_per_turn_shell_limit() {
        let budget = CommandBudget::new();
        for _ in 0..MAX_SHELL_CALLS_PER_TURN {
            // Need to reset throttle between calls for this test
            *budget.last_command_time.lock().unwrap() = None;
            assert!(budget.check_budget(&RiskLevel::Safe).is_ok());
            budget.record_command(&RiskLevel::Safe);
        }
        *budget.last_command_time.lock().unwrap() = None;
        assert!(budget.check_budget(&RiskLevel::Safe).is_err());
    }

    #[test]
    fn test_budget_reset() {
        let budget = CommandBudget::new();
        // Exhaust caution budget
        for _ in 0..MAX_CAUTION_PER_SESSION {
            budget.start_turn();
            *budget.last_command_time.lock().unwrap() = None;
            let _ = budget.check_budget(&RiskLevel::Caution);
            budget.record_command(&RiskLevel::Caution);
        }
        assert!(budget.check_budget(&RiskLevel::Caution).is_err());

        // Reset
        budget.reset();
        budget.start_turn();
        *budget.last_command_time.lock().unwrap() = None;
        assert!(budget.check_budget(&RiskLevel::Caution).is_ok());
    }

    #[test]
    fn test_budget_status() {
        let budget = CommandBudget::new();
        let status = budget.status();
        assert!(status.contains("caution: 0/"));
        assert!(status.contains("danger: 0/"));
    }

    #[test]
    fn test_global_budget() {
        let b1 = get_budget();
        let b2 = get_budget();
        assert!(std::ptr::eq(b1, b2)); // Same instance
    }
}
