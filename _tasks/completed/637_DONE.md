# Task 637: Prompt Input Controller And Command Mode Boundaries

**Status:** pending
**Priority:** HIGH
**Type:** Architecture / UI Internals
**Scope:** `src/claude_ui/claude_input.rs`, `src/input_parser.rs`, `src/app_chat_handlers.rs`, `src/ui_terminal.rs`
**Source:** old pending tasks 013/014, deferred keybinding task 346, user priority on input prompt separation

## Summary

Separate prompt editing, slash commands, file mentions, shell/background modes, and keybinding chords into an input controller that emits typed commands.

## Evidence And Gap

- `claude_input.rs` contains hardcoded slash commands and picker logic.
- `ui_terminal.rs` owns chord timing, queued submissions, mouse mode, autocomplete application, and permission channel state.
- The user specifically asked for separation of concerns between the input prompt and other CLI UI sections.

## Implementation Plan

1. Add an `InputController` owning text buffer, cursor movement, history, picker state, and command-mode parsing.
2. Replace hardcoded slash command arrays with a typed command registry wired to `app_chat_handlers`.
3. Emit `InputAction` events such as submit chat, submit shell, open modal, switch model, toggle reasoning, and cancel.
4. Keep chord/keybinding configuration separate from command execution.
5. Add tests for multiline editing, cursor movement, slash query filtering, file mention selection, and mode transitions.

## Acceptance Criteria

- [ ] `TerminalUI` does not directly parse prompt text into command modes.
- [ ] Slash commands and keybindings are discoverable through one registry.
- [ ] UI tests prove the input prompt can be exercised without a terminal backend.
- [ ] No keyword-based routing is introduced for user intent classification.

## Verification Plan

Run targeted input/controller tests and manually verify `/help`, `/models`, `!shell`, `@file`, multiline submit, and cancel behavior.

