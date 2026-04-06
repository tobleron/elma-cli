# Task 129: Background Sessions with Auto-Approval

## Priority
**Postponed (Tier B — Future Capability)**
**Created:** 2026-04-05
**Status:** Postponed
**Depends on:** Tasks 126 (Daemon Mode), 127 (Telegram Bot)

## Overview

When Elma runs as a daemon with Telegram (or other channels), there's no interactive terminal for y/N confirmations. Background sessions need an approval policy that auto-approves safe commands while blocking or deferring dangerous ones.

## Scope

### 1. Approval Policy Enum
```rust
pub(crate) enum ApprovalPolicy {
    Interactive,  // CLI TUI — ask user (current behavior)
    AutoApprove,  // Daemon/Telegram — approve all tools
    AutoDeny,     // Reject all tools (read-only mode)
    SafeOnly,     // Auto-approve shell/read/search, deny respond-less-safe ops
}
```

### 2. Per-Channel Policy Configuration
- CLI sessions → `Interactive`
- Telegram sessions → `AutoApprove` (with pre-flight safety hooks from Tasks 116-120)
- Configurable via `config.toml` or `/policy` command on Telegram

### 3. Dangerous Command Handling in Auto-Approve Mode
- Pre-flight hooks (Task 123) still run — detect destructive commands
- Instead of asking user: log warning + feed guidance to model
- Model self-corrects based on error feedback
- Post-tool hooks (Task 125) verify results

### 4. Integration Points
- `src/approval_policy.rs` (new) — policy enum + per-channel config
- `src/tool_calling.rs` — `exec_shell()` checks current session's policy
- `src/telegram_bot.rs` — sets `AutoApprove` for Telegram sessions

## Estimated Effort
~200 lines. Half-day focused work.

## Verification
1. `cargo build` clean
2. CLI session → destructive command prompts y/N
3. Telegram session → same command auto-approved, pre-flight logs warning
4. Model self-corrects after pre-flight error feedback in Telegram session
