# Task 109: Implement Indicatif Engine for Advanced Progress & Spinners

## Objective
Deploy the `indicatif` engine to handle thread-safe, high-resolution spinners and progress bars for tracking LLM thinking and tool execution.

## Technical Implementation Plan

### 1. MultiProgress Management
- Initialize a global `MultiProgress` manager in `src/ui_state.rs`.
- Implement a `SpinnerManager` that can spawn, update, and finish multiple concurrent spinners (essential for multi-agent tasks).

### 2. Custom Styling & Templates
- Define Elma-specific templates in `src/ui_colors.rs`:
    - **Spinner Template**: `"{prefix:.bold.dim} {spinner} {msg}"` using Braille frames.
    - **Progress Bar Template**: `"{prefix:.bold} [{bar:40.cyan/blue}] {pos}/{len} {msg}"` for token counts.
- Use `ProgressStyle` to match the "soft gold" and "soft blue" aesthetic of the project.

### 3. Integration with Intel Units
- Update `src/intel_trait.rs` to allow Intel Units to report "Verbs" (Thinking, Analyzing, etc.) to the `MultiProgress` manager.
- Ensure spinners are automatically cleared or "steady" when the agent is waiting for user input.

### 4. Concurrency & Tick Management
- Use `tokio::spawn` to drive the spinner updates independently of the main orchestration logic, ensuring smooth animations even during heavy computation.

## Verification Strategy
1. **Fluidity**: Confirm the Braille spinner is perfectly smooth (no jitter).
2. **Stacking**: Verify that when two agents work in parallel, two spinners appear stacked and update independently.
3. **Cleanup**: Confirm that progress bars are properly cleared from the terminal once a task completes.
