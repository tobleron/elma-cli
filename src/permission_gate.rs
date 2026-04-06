//! @efficiency-role: domain-logic
//!
//! Permission Gate (Task 117)
//!
//! Interactive y/N confirmation for destructive commands.
//! - Default-deny: user must explicitly approve
//! - Session-aware approval caching (approve once, reuse)
//! - Non-interactive mode: auto-deny with guidance

use crate::shell_preflight::{classify_command, RiskLevel};
use crate::ui_colors::*;
use crate::*;
use std::collections::HashSet;

/// Tracks which command patterns have been approved in this session.
struct ApprovalCache {
    /// Approved command prefixes/patterns (e.g., "mv probe_parsing.sh stress_testing/")
    approved: HashSet<String>,
    /// Whether we're in non-interactive mode (piped input, scripts)
    non_interactive: bool,
}

impl ApprovalCache {
    fn new(non_interactive: bool) -> Self {
        Self {
            approved: HashSet::new(),
            non_interactive,
        }
    }

    /// Check if a command pattern has been approved.
    fn is_approved(&self, command: &str) -> bool {
        // Exact match
        if self.approved.contains(command) {
            return true;
        }
        // Prefix match (e.g., "mv *.sh " covers all mv *.sh operations)
        for pattern in &self.approved {
            if command.starts_with(pattern) {
                return true;
            }
        }
        false
    }

    /// Approve a command pattern for future use in this session.
    fn approve(&mut self, command: &str) {
        self.approved.insert(command.to_string());
    }
}

static PERMISSION_CACHE: OnceLock<Mutex<ApprovalCache>> = OnceLock::new();

fn approval_cache() -> &'static Mutex<ApprovalCache> {
    PERMISSION_CACHE.get_or_init(|| {
        // Detect non-interactive mode: stdin is not a terminal
        let non_interactive = !io::stdin().is_terminal();
        Mutex::new(ApprovalCache::new(non_interactive))
    })
}

/// Reset the approval cache (called on /reset or new session).
pub(crate) fn reset_permission_cache() {
    if let Ok(mut cache) = approval_cache().lock() {
        cache.approved.clear();
    }
}

/// Check if a command requires permission gate approval.
/// Returns true if the command is safe to execute (either no approval needed or user approved).
pub(crate) fn check_permission(args: &Args, command: &str) -> bool {
    let risk = classify_command(command);

    // Safe commands pass without approval
    if matches!(risk, RiskLevel::Safe) {
        return true;
    }

    // Check approval cache
    if approval_cache().lock().ok().map(|c| c.is_approved(command)).unwrap_or(false) {
        return true;
    }

    // Non-interactive mode: auto-deny with trace message
    if approval_cache().lock().ok().map(|c| c.non_interactive).unwrap_or(false) {
        trace(args, &format!(
            "permission_gate: DENIED (non-interactive mode): {}",
            command
        ));
        return false;
    }

    // Interactive mode: ask user
    ask_permission(args, command, &risk)
}

/// Record that the user approved this command (for session caching).
pub(crate) fn record_approval(command: &str) {
    if let Ok(mut cache) = approval_cache().lock() {
        cache.approve(command);
    }
}

/// Ask the user for permission to execute a dangerous command.
fn ask_permission(args: &Args, command: &str, risk: &RiskLevel) -> bool {
    let reason = match risk {
        RiskLevel::Dangerous(r) => format!("DANGEROUS: {}", r),
        RiskLevel::Caution => "Caution: this command may have side effects".to_string(),
        RiskLevel::Safe => return true, // Already handled above
    };

    // Disable raw mode temporarily for reading permission (safe to call even if not in raw mode)
    let _ = crossterm::terminal::disable_raw_mode();

    eprintln!();
    eprintln!("  {}", warn_yellow(&reason));
    eprintln!();
    eprintln!("  Command: {}", warn_yellow(command));
    eprintln!();

    // Ask: [y/N] with default N
    eprint!("  Proceed? [y/N] ");
    let _ = io::stderr().flush();

    let mut input = String::new();
    let read_result = io::stdin().read_line(&mut input);

    eprintln!();

    let approved = match read_result {
        Ok(0) => {
            // EOF — treat as deny
            trace(args, "permission_gate: DENIED (EOF)");
            false
        }
        Ok(_) => {
            let answer = input.trim().to_lowercase();
            if answer == "y" || answer == "yes" {
                // Record approval for future similar commands
                record_approval(command);
                trace(args, "permission_gate: APPROVED by user");
                true
            } else {
                trace(args, "permission_gate: DENIED by user");
                false
            }
        }
        Err(e) => {
            eprintln!("  Error reading input: {}", e);
            false
        }
    };

    // Re-enable raw mode (needed when called from TUI mode)
    let _ = crossterm::terminal::enable_raw_mode();

    approved
}

/// Get a display string for the approval cache (for debug/status).
pub(crate) fn approval_cache_summary() -> String {
    approval_cache()
        .lock()
        .ok()
        .map(|cache| {
            if cache.approved.is_empty() {
                "no approvals cached".to_string()
            } else {
                let items: Vec<String> = cache.approved.iter().take(5).cloned().collect();
                format!("{} approved: {}", cache.approved.len(), items.join(", "))
            }
        })
        .unwrap_or_else(|| "cache unavailable".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_approval_cache_exact_match() {
        let mut cache = ApprovalCache::new(true);
        cache.approve("mv file.txt dest/");
        assert!(cache.is_approved("mv file.txt dest/"));
        assert!(!cache.is_approved("mv other.txt dest/"));
    }

    #[test]
    fn test_approval_cache_prefix_match() {
        let mut cache = ApprovalCache::new(true);
        cache.approve("mv *.sh ");
        assert!(cache.is_approved("mv *.sh dest/"));
        assert!(cache.is_approved("mv *.sh backup/"));
    }

    #[test]
    fn test_approval_cache_empty() {
        let cache = ApprovalCache::new(true);
        assert!(!cache.is_approved("rm file.txt"));
    }

    #[test]
    fn test_non_interactive_flag() {
        let cache = ApprovalCache::new(true);
        assert!(cache.non_interactive);
    }

    #[test]
    fn test_interactive_flag() {
        let cache = ApprovalCache::new(false);
        assert!(!cache.non_interactive);
    }
}
