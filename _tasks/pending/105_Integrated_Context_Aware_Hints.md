# Task 105: Integrated Context-Aware Hints

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

## Objective
Provide dynamic keyboard shortcut reminders that appear only when relevant to the user's current context.

## Technical Implementation Plan (Rust)

### Core Requirements
1. **Hint Registry**:
    - Implement a `ContextHint` enum in `src/ui.rs`.
    - Variants: `NavigatingHistory`, `EditingPrompt`, `AwaitingInput`, `ViewingDiff`, `TaskInProgress`.
2. **Context Tracker**:
    - Update `src/ui_state.rs` to maintain a `UIContext` stack.
    - Push context when a mode (e.g., Select Menu, Diff View) starts, pop when it ends.
3. **Rendering Component**:
    - Implement a `draw_context_hints()` in `src/ui.rs`.
    - Display relevant shortcuts (e.g., `Esc: cancel`, `Enter: select`, `Ctrl+O: expand`) in a dedicated area below the prompt or in the Status Line (Task 098).
4. **Integration**:
    - Hook into `src/app_chat_loop.rs` to update context as the user interacts with the app.

### Proposed Rust Dependencies
- Use `src/ui_colors.rs` for subtle coloring of shortcuts (e.g., `ansi_grey`).

### Verification Strategy
1. **Behavior**: 
    - Confirm hints for history navigation appear only when using arrows/Vim keys in history.
    - Confirm the hint for "Ctrl+O" appears only when a diff is visible.
2. **Visuals**:
    - Ensure hints do not clutter the screen or overlap with the status line.
    - Confirm they are easy to read but visually distinct from user text.
3. **Customization**:
    - Allow users to toggle hints in their `profiles.toml`.
    - Support remapping shortcuts in the future.
    - Ensure hints update if the user has custom keybindings.
