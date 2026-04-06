# Task 105: Integrated Context-Aware Hints

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
