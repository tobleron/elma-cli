# 145 Contextual Hints And Tips System

## Continuation Checklist
- [ ] Re-read this task and all linked source/task references before editing.
- [ ] Confirm the task is still valid against current `_tasks/TASKS.md`, `AGENTS.md`, and active master plans.
- [ ] Move or keep this task in `_tasks/active/` before implementation work begins.
- [ ] Inspect the current code/config/docs touched by this task and note any drift from the written plan.
- [ ] Implement the smallest coherent change set that satisfies the next unchecked item.
- [ ] Add or update focused tests, probes, fixtures, or snapshots for the changed behavior.
- [ ] Run `cargo fmt --check` and fix formatting issues.
- [ ] Run `cargo build` and resolve all build errors or warnings introduced by this task.
- [ ] Run targeted `cargo test` commands and any task-specific probes listed below.
- [ ] Run real CLI or pseudo-terminal verification for any user-facing behavior.
- [ ] Record completed work, verification output, and remaining gaps in this task before stopping.
- [ ] Ask for sign-off before moving this task to `_tasks/completed/`.

## Summary
Implement contextual hints that appear based on current context and user behavior.

## Reference
- Claude Code: `_stress_testing/_claude_code_src/services/tips/`
- 50+ contextual tips with relevance checking

## Implementation

### 1. Hint Registry
File: `src/hints.rs` (new)
```rust
pub struct Hint {
    pub id: String,
    pub message: String,
    pub context: HintContext,
    pub cooldown: Duration,
}

pub enum HintContext {
    Always,
    FirstUse,
    Error(String),      // Error pattern
    Mode(String),      // Active mode
    Command(String),  // After command
}
```

### 2. Built-in Hints
Directory: `config/hints/` (new)
- `first_use.md` - Welcome hints for new users
- `mode_switch.md` - Hint after mode switch
- `error_recovery.md` - Hints on common errors
- `context_window.md` - Hint when context is large
- `checkpoint_recovery.md` - Hint for crash recovery

### 3. Hint Engine
File: `src/hints/engine.rs` (new)
- `choose_hint(context)` - Select best hint
- `show_hint(hint)` - Display in UI
- Track shown hints per session
- Respect cooldown periods

### 4. UI Integration
File: `src/ui/hints.rs` (new)
- Display hints in transcript (dimmed, subtle)
- Auto-dismiss on user input
- Configurable enable/disable

## Verification
- [ ] `cargo build` passes
- [ ] Hint selection works
- [ ] Cooldowns are respected