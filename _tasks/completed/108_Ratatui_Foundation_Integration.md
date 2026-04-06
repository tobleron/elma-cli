# Task 108: Integrate Ratatui for UI Foundation & Layout Management

## Objective
Establish a robust UI foundation using `Ratatui` to manage complex terminal layouts, including the persistent status line, interactive trees, and multi-pane views.

## Technical Implementation Plan

### 1. Terminal Backend Setup
- Integrate `crossterm` as the primary backend for `Ratatui`.
- Implement a `TerminalGuard` in `src/ui.rs` that:
    - Enables "Raw Mode" on startup.
    - Enters the "Alternate Screen" for full-app modes (like menus) or stays on the main screen for "inline" rendering.
    - Implements `Drop` to ensure the terminal is restored even if Elma crashes.

### 2. Frame & Layout Architecture
- Update `src/ui_state.rs` to support a `LayoutManager`.
- Define a standard `AppLayout` using Ratatui's `Layout` and `Constraint` modules:
    - **Header**: Session name & model (Top).
    - **Body**: Scrollable chat history / Task tree (Center).
    - **Footer**: Persistent Status Line & Context Bar (Bottom).

### 3. Rendering Loop Integration
- Implement an `ui_render_tick()` function called by `src/app_chat_loop.rs`.
- Use `terminal.draw(|f| { ... })` to render widgets based on current state.
- Support "Partial Rendering" where only the status line updates to save CPU on local models.

### 4. Integration with Existing Files
- **src/ui_chat.rs**: Adapt existing print statements to be rendered within a Ratatui `Paragraph` widget.
- **src/ui_state.rs**: Add `current_frame` and `terminal_size` to the global state.

## Verification Strategy
1. **Layout Integrity**: Verify that the status line stays anchored at the bottom during long text outputs.
2. **Resizing**: Confirm the `Layout` recalculates correctly when the terminal window is resized.
3. **Performance**: Ensure the CPU usage remains low by throttling the render loop (e.g., 20 FPS).
