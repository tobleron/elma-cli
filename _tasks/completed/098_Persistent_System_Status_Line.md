# Task 098: Persistent System Status Line

## Objective
Implement a persistent, single-line status bar at the bottom of the terminal that provides real-time updates on session metadata without interfering with the scrollable chat history.

## Technical Implementation Plan (Rust)

### Core Requirements
1. **Screen Management**: Since Elma does not currently use `ratatui`, use `crossterm` to handle terminal resizing, cursor positioning, and "scrolling region" (DECSTBM) to reserve the last line for the status bar.
2. **State Management**: Update `src/ui_state.rs` to track live metadata:
    - Current Model ID/Name.
    - Active Profile.
    - Current Session Duration.
    - Total Session Cost (from `src/metrics.rs`).
    - Current Working Directory (shortened).
3. **Rendering Loop**:
    - Implement a `draw_status_line()` function in `src/ui.rs`.
    - Use ANSI escape codes (or `crossterm`) to:
        - Save cursor position.
        - Move to the last line of the terminal.
        - Clear the line (`\x1b[2K`).
        - Print the formatted status string with `src/ui_colors.rs` helpers.
        - Restore cursor position.
4. **Integration**:
    - Call `draw_status_line()` after every significant state change (e.g., after an Intel Unit returns, after a tool execution).
    - Handle `SIGWINCH` (terminal resize) to recalculate the status line width.

### Proposed Rust Dependencies
- `crossterm = "0.27"`: For terminal manipulation and cursor control.

### Verification Strategy
1. **Build**: `cargo build` must succeed with zero warnings.
2. **Behavior**: 
    - Verify the status line stays at the bottom during long outputs.
    - Verify that resizing the terminal correctly repositions the status line.
    - Verify that metadata (model, cost) updates in real-time.
3. **Compatibility**: Ensure it doesn't break existing `ui_chat.rs` logic.
