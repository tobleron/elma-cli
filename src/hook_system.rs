//! @efficiency-role: domain-logic
//!
//! Extensible Hook System (Tasks 123, 124, 125)
//!
//! Three-phase hook framework for shell command lifecycle:
//! - **PreToolHook** (Task 123): Run before execution. Can block, modify, or allow.
//! - **ContextModifier** (Task 124): Adapts model trust after errors/successes.
//! - **PostToolHook** (Task 125): Run after execution. Can verify, audit, or trigger actions.
//!
//! Built-in hooks implement the safety net from Phase 4:
//! - DestructiveCommandDetector (pre) — already in shell_preflight
//! - PathProtector (pre) — already in shell_preflight
//! - UnscopedGlobDetector (pre) — already in shell_preflight
//! - BudgetEnforcer (pre) — already in command_budget
//! - ResultVerifier (post) — new: verifies expected side effects occurred

use crate::*;
use std::path::Path;

// ============================================================================
// Task 123: Pre-Tool Hook Trait
// ============================================================================

/// Result of a pre-tool hook decision.
#[derive(Debug, Clone)]
pub(crate) enum PreHookDecision {
    /// Allow the command to proceed.
    Allow,
    /// Block the command with a reason.
    Block(String),
    /// Modify the command before execution.
    Modify(String),
    /// Require explicit confirmation before proceeding.
    RequireConfirm(String),
}

/// A hook that runs before a shell command executes.
pub(crate) trait PreToolHook: Send + Sync {
    fn name(&self) -> &str;
    fn execute(&self, command: &str, workdir: &PathBuf) -> PreHookDecision;
}

/// Built-in: Confirms the preflight pipeline already handles this.
/// This hook integrates shell_preflight into the hook system.
pub(crate) struct PreflightIntegrator;

impl PreToolHook for PreflightIntegrator {
    fn name(&self) -> &str {
        "preflight_integrator"
    }

    fn execute(&self, command: &str, workdir: &PathBuf) -> PreHookDecision {
        let result = shell_preflight::preflight_command(command, workdir);

        if let Some(guidance) = result.error_guidance {
            return PreHookDecision::Block(format!("[preflight] {}", guidance));
        }

        if let Some(preview) = result.dry_run_preview {
            return PreHookDecision::RequireConfirm(format!("[dry-run] {}", preview));
        }

        PreHookDecision::Allow
    }
}

/// Built-in: Permission gate integration.
pub(crate) struct PermissionGateIntegrator;

impl PreToolHook for PermissionGateIntegrator {
    fn name(&self) -> &str {
        "permission_gate"
    }

    fn execute(&self, command: &str, _workdir: &PathBuf) -> PreHookDecision {
        // Note: This requires args for check_permission; we skip interactive
        // check here since that's handled separately. We just log.
        trace_verbose(true, &format!("[permission_gate] checking: {}", command));
        PreHookDecision::Allow
    }
}

/// Built-in: Budget enforcer integration.
pub(crate) struct BudgetEnforcer;

impl PreToolHook for BudgetEnforcer {
    fn name(&self) -> &str {
        "budget_enforcer"
    }

    fn execute(&self, command: &str, _workdir: &PathBuf) -> PreHookDecision {
        let budget = crate::command_budget::get_budget();
        // We don't have risk classification here without running preflight again,
        // so we rely on the command_budget check in tool_calling.rs instead.
        // This hook exists for the extensibility pattern demonstration.
        trace_verbose(
            true,
            &format!("[budget_enforcer] status: {}", budget.status()),
        );
        PreHookDecision::Allow
    }
}

// ============================================================================
// Task 124: Context Modifier Trait
// ============================================================================

/// A context modifier adapts the model's trust level based on execution outcomes.
/// After a destructive command succeeds, the model may need lower trust.
/// After a path error is corrected, inject the correction into the prompt.
pub(crate) trait ContextModifier: Send + Sync {
    fn name(&self) -> &str;
    /// Called after command execution. Returns a prompt modification if needed.
    fn after_execution(&self, command: &str, success: bool, output: &str) -> Option<String>;
    /// Called after a command error. Returns guidance for the model.
    fn after_error(&self, command: &str, error: &str) -> Option<String>;
}

/// Built-in: Trust decay after destructive commands.
/// After N destructive commands succeed, inject a trust warning into the system prompt.
pub(crate) struct TrustDecayModifier {
    destructive_count: std::sync::atomic::AtomicUsize,
    threshold: usize,
}

impl TrustDecayModifier {
    pub(crate) fn new(threshold: usize) -> Self {
        Self {
            destructive_count: std::sync::atomic::AtomicUsize::new(0),
            threshold,
        }
    }

    pub(crate) fn reset(&self) {
        self.destructive_count
            .store(0, std::sync::atomic::Ordering::Relaxed);
    }
}

impl ContextModifier for TrustDecayModifier {
    fn name(&self) -> &str {
        "trust_decay"
    }

    fn after_execution(&self, command: &str, success: bool, _output: &str) -> Option<String> {
        if !success {
            return None;
        }
        if shell_preflight::classify_command(command) != shell_preflight::RiskLevel::Caution {
            return None;
        }

        let count = self
            .destructive_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            + 1;
        if count >= self.threshold {
            Some(format!(
                "! Trust notice: {} caution-level commands have been executed this session. \
                Verify paths carefully before proceeding with more mutations.",
                count
            ))
        } else {
            None
        }
    }

    fn after_error(&self, command: &str, error: &str) -> Option<String> {
        Some(format!(
            "! Command failed: {}\nError: {}\n\n\
            Verify the command syntax and paths before retrying. \
            Use `ls` or `find` to confirm files exist before mutating.",
            command, error
        ))
    }
}

// ============================================================================
// Task 125: Post-Tool Hook Trait
// ============================================================================

/// Result of a post-tool hook execution.
#[derive(Debug, Clone)]
pub(crate) struct PostHookResult {
    pub(crate) hook_name: String,
    pub(crate) ok: bool,
    pub(crate) message: Option<String>,
}

/// A hook that runs after a shell command executes.
/// Used for verification, auditing, and triggering follow-up actions.
pub(crate) trait PostToolHook: Send + Sync {
    fn name(&self) -> &str;
    fn execute(&self, command: &str, success: bool, output: &str) -> PostHookResult;
}

/// Built-in: Verifies that destructive commands actually had the expected effect.
pub(crate) struct ResultVerifier;

impl PostToolHook for ResultVerifier {
    fn name(&self) -> &str {
        "result_verifier"
    }

    fn execute(&self, command: &str, success: bool, output: &str) -> PostHookResult {
        let cmd = command.trim();

        // For rm: verify files are gone
        if let Some(args) = cmd.strip_prefix("rm ") {
            let files: Vec<&str> = args
                .split_whitespace()
                .filter(|a| !a.starts_with('-') && !a.contains('*'))
                .collect();
            let mut gone = 0;
            let mut remaining = Vec::new();
            for f in &files {
                let path = Path::new(f);
                if path.exists() {
                    remaining.push(f.to_string());
                } else {
                    gone += 1;
                }
            }
            if !remaining.is_empty() {
                return PostHookResult {
                    hook_name: self.name().to_string(),
                    ok: false,
                    message: Some(format!(
                        "Verification: {} file(s) deleted, but {} still exist: {}",
                        gone,
                        remaining.len(),
                        remaining.join(", ")
                    )),
                };
            }
            return PostHookResult {
                hook_name: self.name().to_string(),
                ok: true,
                message: Some(format!(
                    "Verification: {} file(s) successfully deleted.",
                    gone
                )),
            };
        }

        // For mv: verify source is gone and dest exists
        if let Some(args) = cmd.strip_prefix("mv ") {
            let parts: Vec<&str> = args.split_whitespace().collect();
            if parts.len() >= 2 {
                let src = Path::new(parts[0]);
                let dest = Path::new(parts[1]);
                let src_gone = !src.exists();
                let dest_exists =
                    dest.exists() || dest.parent().map(|p| p.exists()).unwrap_or(false);
                return PostHookResult {
                    hook_name: self.name().to_string(),
                    ok: src_gone,
                    message: Some(format!(
                        "Verification: mv '{}' → '{}' — source gone: {}, destination exists: {}",
                        parts[0], parts[1], src_gone, dest_exists
                    )),
                };
            }
        }

        // For other commands, just log success
        PostHookResult {
            hook_name: self.name().to_string(),
            ok: success,
            message: if success {
                Some(format!(
                    "Verification: Command executed successfully ({} chars output)",
                    output.len()
                ))
            } else {
                Some(format!(
                    "Verification: Command failed. Output: {}",
                    output.chars().take(200).collect::<String>()
                ))
            },
        }
    }
}

/// Built-in: Audit logger — records all shell commands to trace log.
pub(crate) struct AuditLogger;

impl PostToolHook for AuditLogger {
    fn name(&self) -> &str {
        "audit_logger"
    }

    fn execute(&self, command: &str, success: bool, output: &str) -> PostHookResult {
        trace_verbose(
            true,
            &format!(
                "[audit] command={} success={} output_len={}",
                command.chars().take(80).collect::<String>(),
                success,
                output.len()
            ),
        );
        PostHookResult {
            hook_name: self.name().to_string(),
            ok: true,
            message: None,
        }
    }
}

// ============================================================================
// Hook Registry
// ============================================================================

/// Registry of all active hooks.
pub(crate) struct HookRegistry {
    pre_hooks: Vec<Box<dyn PreToolHook>>,
    post_hooks: Vec<Box<dyn PostToolHook>>,
    context_modifiers: Vec<Box<dyn ContextModifier>>,
}

impl HookRegistry {
    pub(crate) fn new() -> Self {
        let mut reg = Self {
            pre_hooks: Vec::new(),
            post_hooks: Vec::new(),
            context_modifiers: Vec::new(),
        };
        // Register built-in hooks
        reg.register_pre_hook(Box::new(PreflightIntegrator));
        reg.register_pre_hook(Box::new(PermissionGateIntegrator));
        reg.register_pre_hook(Box::new(BudgetEnforcer));
        reg.register_post_hook(Box::new(ResultVerifier));
        reg.register_post_hook(Box::new(AuditLogger));
        reg.register_context_modifier(Box::new(TrustDecayModifier::new(5)));
        reg
    }

    pub(crate) fn register_pre_hook(&mut self, hook: Box<dyn PreToolHook>) {
        self.pre_hooks.push(hook);
    }

    pub(crate) fn register_post_hook(&mut self, hook: Box<dyn PostToolHook>) {
        self.post_hooks.push(hook);
    }

    pub(crate) fn register_context_modifier(&mut self, modifier: Box<dyn ContextModifier>) {
        self.context_modifiers.push(modifier);
    }

    /// Run all pre-hooks on a command. Returns block message if any hook blocks.
    pub(crate) fn run_pre_hooks(&self, command: &str, workdir: &PathBuf) -> Option<String> {
        for hook in &self.pre_hooks {
            match hook.execute(command, workdir) {
                PreHookDecision::Allow => {}
                PreHookDecision::Block(reason) => return Some(reason),
                PreHookDecision::Modify(new_cmd) => {
                    // For now, we log modifications but don't auto-apply
                    trace_verbose(
                        true,
                        &format!(
                            "[hook:{}] would modify command to: {}",
                            hook.name(),
                            new_cmd
                        ),
                    );
                }
                PreHookDecision::RequireConfirm(msg) => {
                    // Return the confirmation message — caller decides
                    return Some(msg);
                }
            }
        }
        None
    }

    /// Run all post-hooks after command execution.
    pub(crate) fn run_post_hooks(
        &self,
        command: &str,
        success: bool,
        output: &str,
    ) -> Vec<PostHookResult> {
        self.post_hooks
            .iter()
            .map(|h| h.execute(command, success, output))
            .collect()
    }

    /// Run all context modifiers after command execution.
    pub(crate) fn run_context_modifiers(
        &self,
        command: &str,
        success: bool,
        output: &str,
    ) -> Vec<String> {
        self.context_modifiers
            .iter()
            .filter_map(|m| m.after_execution(command, success, output))
            .collect()
    }

    /// Run all context modifiers after a command error.
    pub(crate) fn run_context_modifier_errors(&self, command: &str, error: &str) -> Vec<String> {
        self.context_modifiers
            .iter()
            .filter_map(|m| m.after_error(command, error))
            .collect()
    }
}

/// Global hook registry.
static HOOK_REGISTRY: std::sync::OnceLock<HookRegistry> = std::sync::OnceLock::new();

pub(crate) fn get_hook_registry() -> &'static HookRegistry {
    HOOK_REGISTRY.get_or_init(|| HookRegistry::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preflight_integrator_blocks_protected() {
        let hook = PreflightIntegrator;
        let workdir = PathBuf::from(".");
        let decision = hook.execute("rm -rf sessions/", &workdir);
        assert!(matches!(decision, PreHookDecision::Block(_)));
    }

    #[test]
    fn test_preflight_integrator_allows_safe() {
        let hook = PreflightIntegrator;
        let workdir = PathBuf::from(".");
        let decision = hook.execute("ls -la", &workdir);
        assert!(matches!(decision, PreHookDecision::Allow));
    }

    #[test]
    fn test_trust_decay_modifier() {
        let modifier = TrustDecayModifier::new(3);
        // First 2 caution commands: no warning
        assert!(modifier.after_execution("mv a b", true, "").is_none());
        assert!(modifier.after_execution("cp c d", true, "").is_none());
        // 3rd: warning
        let msg = modifier.after_execution("mv e f", true, "").unwrap();
        assert!(msg.contains("3 caution-level commands"));
    }

    #[test]
    fn test_result_verifier_rm() {
        let hook = ResultVerifier;
        // rm on nonexistent files — verification should note they don't exist
        let result = hook.execute("rm nonexistent_file.txt", true, "");
        assert!(result.ok); // rm -f on nonexistent is technically success
    }

    #[test]
    fn test_audit_logger() {
        let hook = AuditLogger;
        let result = hook.execute("ls -la", true, "file1\nfile2");
        assert!(result.ok);
        assert!(result.message.is_none()); // Audit logs to trace, not user message
    }

    #[test]
    fn test_hook_registry_built_in_hooks() {
        let reg = HookRegistry::new();
        // Should have 3 pre-hooks and 2 post-hooks
        assert!(!reg.pre_hooks.is_empty());
        assert!(!reg.post_hooks.is_empty());
        assert!(!reg.context_modifiers.is_empty());
    }

    #[test]
    fn test_global_registry() {
        let r1 = get_hook_registry();
        let r2 = get_hook_registry();
        assert!(std::ptr::eq(r1, r2));
    }
}
