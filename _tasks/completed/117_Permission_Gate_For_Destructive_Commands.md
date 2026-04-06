# Task 117: Permission Gate for Destructive Commands

## Priority
**P0 — Critical Safety**
**Created:** 2026-04-05
**Status:** Pending
**Dependencies:** Task 116 (Destructive Command Detection)

## Problem

Destructive commands execute immediately without user confirmation. The model can (and does) generate `mv`, `rm`, and pipeline-destruct operations without understanding the consequences.

## Scope

### 1. Interactive Confirmation
- When a destructive command is detected (Task 116), pause execution and prompt user
- Format: `⚠ Destructive: mv *.sh stress_testing/ (12 files). Proceed? [y/N]`
- Default: `N` (deny) — user must explicitly confirm
- Non-interactive mode (piped input, scripts): auto-deny destructive commands with error message

### 2. Session-Aware Permissions
- Track which commands have been approved in the current session
- If user approves `rm test_*.sh`, similar future commands in same session can proceed without re-asking
- Approval expires on session reset

### 3. Integration Points
- `src/tool_calling.rs` — `exec_shell()` checks permission gate before execution
- `src/permission_gate.rs` (new) — approval tracking + prompt logic
- `src/app.rs` — `--no-confirm` flag for automated/scripted use

## Design Principles
- **Offline-first:** No network needed for permission decisions
- **Small-model-friendly:** User confirms at terminal level, not via another LLM call
- **Principle-first:** Default-deny for destructive, default-allow for safe
- **Non-blocking:** Safe commands never wait for confirmation

## Verification
1. `cargo build` clean
2. `cargo test` — permission tracking, approval caching, non-interactive mode
3. Real CLI: `rm *.sh` → prompts user, waits for y/N
4. Real CLI: `ls` → no prompt, executes immediately
5. Real CLI: pipe input mode → destructive commands denied with guidance

## Acceptance Criteria
- [ ] Destructive commands require explicit y/N confirmation
- [ ] Default response is deny (N)
- [ ] Non-interactive mode auto-denies with helpful error
- [ ] Session tracks approved patterns to reduce repetition
- [ ] Safe commands never blocked by permission gate
