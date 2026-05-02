//! @efficiency-role: domain-logic
//!
//! Permission Gate (Task 117)
//!
//! Interactive y/N confirmation for destructive commands.
//! - Default-deny: user must explicitly approve
//! - Session-aware approval caching (approve once, reuse)
//! - Non-interactive mode: auto-deny with guidance

use crate::safe_mode::{get_safe_mode, SafeMode};
use crate::shell_preflight::{classify_command, RiskLevel};
use crate::ui_theme::*;
use crate::*;
use std::collections::HashSet;
use std::io::IsTerminal;

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
        // Exact match always wins
        if self.approved.contains(command) {
            return true;
        }
        // Prefix match: allow only if the approved pattern inherently includes
        // a word boundary (ends with space or '/'), or if the command continues
        // with a boundary (space or '/') after the matched prefix.
        'pattern: for pattern in &self.approved {
            if !command.starts_with(pattern.as_str()) {
                continue;
            }
            let pattern = pattern.as_str();
            // If pattern already ends with a boundary char, no further check needed.
            if pattern.ends_with(' ') || pattern.ends_with('/') {
                return true;
            }
            // Otherwise the command must have a boundary immediately after the prefix.
            let rest = &command[pattern.len()..];
            if rest.is_empty() || rest.starts_with(' ') || rest.starts_with('/') {
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
        let non_interactive = !std::io::stdin().is_terminal();
        Mutex::new(ApprovalCache::new(non_interactive))
    })
}

/// Reset the approval cache (called on /reset or new session).
pub(crate) fn reset_permission_cache() {
    let mut cache = approval_cache().lock().unwrap_or_else(|e| e.into_inner());
    cache.approved.clear();
}

/// Check if a command requires permission gate approval.
/// Returns true if the command is safe to execute (either no approval needed or user approved).
/// `is_destructive`: if false, the step is not destructive and permission is auto-granted.
pub(crate) async fn check_permission(
    args: &Args,
    command: &str,
    is_destructive: bool,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> bool {
    // If the step is not destructive, grant permission immediately
    if !is_destructive {
        return true;
    }

    // Capture current turn ID for event recording (if available)
    let turn_id = crate::event_log::get_current_turn_id();

    let risk = classify_command(command);
    let mode = get_safe_mode();

    // Safe mode "off": auto-approve everything except truly dangerous commands
    if mode == SafeMode::Off {
        if !matches!(risk, RiskLevel::Dangerous(_)) {
            trace(
                args,
                &format!(
                    "permission_gate: AUTO-APPROVED (safe_mode=off): {}",
                    command
                ),
            );
            if let Some(ref tid) = turn_id {
                crate::event_log::record_policy_event(
                    crate::event_log::PolicyEventType::PermissionGranted,
                    tid,
                    None,
                    &format!("safe_mode=off auto-approve: {}", command),
                );
            }
            return true;
        }
    }

    // Safe commands pass without approval (unless safe_mode=on)
    if matches!(risk, RiskLevel::Safe) && mode.allows_safe_auto_approve() {
        if let Some(ref tid) = turn_id {
            crate::event_log::record_policy_event(
                crate::event_log::PolicyEventType::PermissionGranted,
                tid,
                None,
                &format!("safe command auto-approve: {}", command),
            );
        }
        return true;
    }

    // Check approval cache (single lock acquisition; recover from poisoning)
    let (already_approved, is_non_interactive) = {
        let cache = approval_cache().lock().unwrap_or_else(|e| e.into_inner());
        (cache.is_approved(command), cache.non_interactive)
    };
    if already_approved {
        if let Some(ref tid) = turn_id {
            crate::event_log::record_policy_event(
                crate::event_log::PolicyEventType::PermissionGranted,
                tid,
                None,
                &format!("cached approval: {}", command),
            );
        }
        return true;
    }

    // Non-interactive mode handling based on safe mode
    if is_non_interactive {
        if mode.allows_non_interactive_auto() && !matches!(risk, RiskLevel::Dangerous(_)) {
            trace(
                args,
                &format!(
                    "permission_gate: ALLOWED (non-interactive, safe_mode={}): {}",
                    mode, command
                ),
            );
            if let Some(ref tid) = turn_id {
                crate::event_log::record_policy_event(
                    crate::event_log::PolicyEventType::PermissionGranted,
                    tid,
                    None,
                    &format!("non-interactive auto-approve: {}", command),
                );
            }
            return true;
        }
        trace(
            args,
            &format!(
                "permission_gate: DENIED (non-interactive mode, safe_mode={}): {}",
                mode, command
            ),
        );
        if let Some(ref tid) = turn_id {
            crate::event_log::record_policy_event(
                crate::event_log::PolicyEventType::PolicyBlocked,
                tid,
                None,
                &format!("non-interactive policy blocked: {}", command),
            );
        }
        return false;
    }

    // If we have a TUI, use async permission request
    if let Some(ref mut t) = tui {
        // Record that we are requesting permission
        if let Some(ref tid) = turn_id {
            crate::event_log::record_policy_event(
                crate::event_log::PolicyEventType::PermissionRequested,
                tid,
                None,
                &format!("TUI permission request: {}", command),
            );
        }
        let approved = t.request_permission(command).await;
        if approved {
            record_approval(command);
            if let Some(ref tid) = turn_id {
                crate::event_log::record_policy_event(
                    crate::event_log::PolicyEventType::PermissionGranted,
                    tid,
                    None,
                    &format!("TUI user approved: {}", command),
                );
            }
        } else {
            if let Some(ref tid) = turn_id {
                crate::event_log::record_policy_event(
                    crate::event_log::PolicyEventType::PermissionDenied,
                    tid,
                    None,
                    &format!("TUI user denied: {}", command),
                );
            }
        }
        return approved;
    }

    // Interactive mode: ask user (blocking for now; modal integration TODO)
    // HACK: To prevent hangs in TUI mode, check if stdin is a TTY.
    // If not a TTY (likely PTY or pipe), deny to avoid blocking.
    // TODO: Properly integrate with modal system.
    if !std::io::stdin().is_terminal() {
        trace(
            args,
            &format!(
                "permission_gate: DENIED (non-TTY stdin, likely TUI/PTY, safe_mode={}): {}",
                mode, command
            ),
        );
        if let Some(ref tid) = turn_id {
            crate::event_log::record_policy_event(
                crate::event_log::PolicyEventType::PolicyBlocked,
                tid,
                None,
                &format!("non-TTY fallback denied: {}", command),
            );
        }
        return false;
    }
    // Record permission request before blocking ask
    if let Some(ref tid) = turn_id {
        crate::event_log::record_policy_event(
            crate::event_log::PolicyEventType::PermissionRequested,
            tid,
            None,
            &format!("blocking ask: {}", command),
        );
    }
    let approved = ask_permission(args, command, &risk);
    if approved {
        if let Some(ref tid) = turn_id {
            crate::event_log::record_policy_event(
                crate::event_log::PolicyEventType::PermissionGranted,
                tid,
                None,
                &format!("blocking ask approved: {}", command),
            );
        }
    } else {
        if let Some(ref tid) = turn_id {
            crate::event_log::record_policy_event(
                crate::event_log::PolicyEventType::PermissionDenied,
                tid,
                None,
                &format!("blocking ask denied: {}", command),
            );
        }
    }
    approved
}

/// Record that the user approved this command (for session caching).
pub(crate) fn record_approval(command: &str) {
    let mut cache = approval_cache().lock().unwrap_or_else(|e| e.into_inner());
    cache.approve(command);
}

/// Ask the user for permission to execute a dangerous command.
fn ask_permission(args: &Args, command: &str, risk: &RiskLevel) -> bool {
    let reason = match risk {
        RiskLevel::Dangerous(r) => format!("DANGEROUS: {}", r),
        RiskLevel::Caution => "Caution: this command may have side effects".to_string(),
        RiskLevel::Safe => return true, // Already handled above
    };

    eprintln!();
    eprintln!("  {}", warn_yellow(&reason));
    eprintln!();
    eprintln!("  Command: {}", warn_yellow(command));
    eprintln!();

    let approved = crate::ui::ui_interact::confirm("Proceed?");

    if approved {
        // Record approval for future similar commands
        record_approval(command);
        trace(args, "permission_gate: APPROVED by user");
    } else {
        trace(args, "permission_gate: DENIED by user");
    }

    approved
}

/// Get a display string for the approval cache (for debug/status).
pub(crate) fn approval_cache_summary() -> String {
    let cache = approval_cache().lock().unwrap_or_else(|e| e.into_inner());
    if cache.approved.is_empty() {
        "no approvals cached".to_string()
    } else {
        let items: Vec<String> = cache.approved.iter().take(5).cloned().collect();
        format!("{} approved: {}", cache.approved.len(), items.join(", "))
    }
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

    #[test]
    fn test_approval_prefix_does_not_bypass_boundary() {
        let mut cache = ApprovalCache::new(false);
        cache.approve("rm /tmp/test");
        assert!(cache.is_approved("rm /tmp/test")); // exact match
        assert!(!cache.is_approved("rm /tmp/test_other")); // underscore ≠ word boundary
        assert!(cache.is_approved("rm /tmp/test file2")); // space boundary → ok
        assert!(cache.is_approved("rm /tmp/test/file2")); // slash boundary → ok
    }
}
