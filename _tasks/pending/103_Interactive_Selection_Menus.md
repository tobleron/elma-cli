# Task 103: Interactive Selection Menus

## Objective
Implement custom, keyboard-navigable interactive menus for selecting models, profiles, and themes.

## Technical Implementation Plan (Rust)

### Core Requirements
1. **Interactive Loop**:
    - Use `crossterm`'s event loop to capture keyboard input (arrows, Enter, Esc, Vim keys).
    - Enable raw mode for immediate input feedback.
2. **Menu State Model**:
    - Create a `SelectMenu` struct in `src/ui_state.rs`.
    - Fields: `options: Vec<String>`, `current_index: usize`, `title: String`, `search_query: String`.
3. **Rendering Component**:
    - Implement a `draw_select_menu(menu: &SelectMenu)` in `src/ui.rs`.
    - Highlight the selected option with a background color (e.g., `ansi_soft_blue`).
    - Support real-time filtering (fuzzy search) as the user types.
4. **Integration**:
    - Use the menu for commands like `/model` or `/profile` where the user needs to choose from a list.
5. **Vim Compatibility**:
    - Support `j/k` for navigation and `/` for search to satisfy power users.

### Proposed Rust Dependencies
- `crossterm`: For input events and raw mode.
- `fuzzy-matcher`: For real-time search filtering.

### Verification Strategy
1. **UX**:
    - Confirm the menu is responsive and keyboard-friendly.
    - Confirm the selection is returned correctly to the calling function.
2. **Robustness**:
    - Verify it handles large lists of models with scrolling.
    - Confirm it handles empty search results gracefully.
3. **Safety**:
    - Ensure raw mode is disabled if the application crashes (via `Drop` implementation or panic hook).
