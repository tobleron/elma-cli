# Task 099: Context Window Usage Visualizer

## Objective
Implement a progress-bar visualizer within the status line to show real-time context usage against the current model's token limit.

## Technical Implementation Plan (Rust)

### Core Requirements
1. **Token Tracking**: Leverage `src/metrics.rs` and `src/app_bootstrap_modes.rs` to track total input/output tokens in the current session.
2. **Model Metadata**: Update model profiles to include the `context_window_size` (e.g., 8k, 32k, 128k).
3. **Progress Bar Component**:
    - Implement a `render_token_bar(current: usize, max: usize, width: usize)` in `src/ui.rs`.
    - Use Unicode block characters (e.g., `\u2588`, `\u2591`) for the progress bar.
    - Color-code the bar:
        - Green (< 70% usage)
        - Yellow (70-90% usage)
        - Red (> 90% usage)
4. **Integration**:
    - Embed the token bar into the Persistent Status Line (Task 098).
    - Update the bar after every turn.

### Proposed Rust Dependencies
- Use existing `src/ui_colors.rs` for bar coloring.

### Verification Strategy
1. **Behavior**: 
    - Simulate high token usage and confirm the bar turns red.
    - Confirm the percentage and numerical counts match `src/metrics.rs`.
2. **Edge Cases**: 
    - Verify behavior when context window is exceeded.
    - Handle cases where the model's context window size is unknown.
