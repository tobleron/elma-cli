# Task 120: Session & Infrastructure Directory Protection

## Priority
**P1 — Safety**
**Created:** 2026-04-05
**Status:** Pending
**Dependencies:** Task 116 (Destructive Command Detection)

## Problem

The model could (theoretically) delete or move files in `sessions/`, `config/`, `_tasks/`, or `src/` — directories critical to Elma's operation. No protection exists.

## Scope

### 1. Protected Directory List
- `sessions/` — user conversation history
- `config/` — model and profile configurations
- `_tasks/` — task management
- `src/` — source code
- `.git/` — version control
- `Cargo.toml`, `Cargo.lock` — build configuration

### 2. Protection Behavior
- Attempted mutation of protected paths: block with specific error
- Error message: "Cannot modify protected path: sessions/. Use /snapshot or explicit override."
- Model receives guidance on why it's blocked and what alternatives exist

### 3. Override Mechanism
- Explicit user confirmation bypasses protection
- Model can request override via special tool parameter

### 4. Integration Points
- `src/shell_preflight.rs` — path protection checks
- `src/tool_calling.rs` — `exec_shell()` validates paths

## Design Principles
- **Truthful:** Explain why path is protected, not just deny
- **Overrideable:** User can always override, but must be explicit
- **Small-model-friendly:** Clear alternative suggestions ("use `ls sessions/` to inspect")

## Verification
1. `cargo build` clean
2. Real CLI: `rm sessions/s_*/` → blocked with guidance
3. Real CLI: `rm config/orchestrator.toml` → blocked
4. Real CLI: `ls sessions/` → allowed (read-only)

## Acceptance Criteria
- [ ] Protected directories cannot be modified by model-initiated commands
- [ ] Specific error message explains protection and alternatives
- [ ] User can explicitly override with confirmation
- [ ] Read-only access (ls, cat, rg) to protected paths works normally
