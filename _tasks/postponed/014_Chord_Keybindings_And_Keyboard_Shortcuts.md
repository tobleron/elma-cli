# 147 Chord Keybindings And Keyboard Shortcuts

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
Implement chord keybinding system (multi-key sequences) for keyboard shortcuts.

## Reference
- Claude Code: `KeybindingContext` with chord sequences (e.g., `Ctrl+K Ctrl+S`)

## Implementation

### 1. Keybinding Schema
File: `src/keybindings.rs` (new)
```rust
pub struct Keybinding {
    pub sequence: Vec<Key>,       // e.g., [Ctrl(K), Ctrl(S)]
    pub action: KeyAction,
    pub context: KeyContext,      // Global, Chat, Footer, etc.
}

pub enum Key {
    Char(char),
    Ctrl(char),
    Alt(char),
    Esc,
    Enter,
    Tab,
    ArrowUp,
    ArrowDown,
    // ...
}

pub enum KeyAction {
    Command(String),
    Mode(String),
    Scroll(ScrollDir),
    // ...
}
```

### 2. Default Keybindings
File: `config/keybindings.toml`
```toml
[global]
"ctrl(c)" = "abort"
"ctrl(z)" = "undo"

[chat]
"ctrl(k)" = "clear"
"ctrl(r)" = "resubmit"

[footer]
"up" = "history_previous"
"down" = "history_next"
"tab" = "accept_suggestion"
"ctrl(c)" = "cancel"
```

### 3. Input Handler
File: `src/ui/input_handler.rs` (new)
- Buffer ongoing key sequences
- Timeout on incomplete chords (500ms)
- Match against registered keybindings
- Emit action events

### 4. Context System
File: `src/ui/keybinding_context.rs` (new)
- Track current input context (chat, selection, footer)
- Resolve keybindings per context
- Switch contexts on UI state change

## Verification
- [ ] `cargo build` passes
- [ ] Single-key shortcuts work
- [ ] Chord sequences work