# Task 107: Visual Effort Indicator

## Objective
Provide a visual representation of the computational complexity and "thinking time" used for each response.

## Technical Implementation Plan (Rust)

### Core Requirements
1. **Effort Metrics**:
    - Update `src/metrics.rs` to track total computation time (wall clock) for a specific turn.
    - Include sub-metrics: `intel_unit_time`, `tool_execution_time`, `llm_wait_time`.
2. **Effort Calculation**:
    - Implement a `calculate_effort_score(turn_data: &TurnMetrics)` in `src/metrics.rs`.
    - Weights should favor "active" time (tools, thinking) over "passive" wait time.
3. **Rendering Component**:
    - Implement a `draw_effort_indicator(score: EffortScore)` in `src/ui.rs`.
    - Display the score as a subtle badge (e.g., `[Effort: 42]`) or an icon-based indicator (e.g., `⚡️⚡️⚡️`).
4. **Integration**:
    - Embed the effort score in the final answer footer or in the Persistent Status Line (Task 098).
5. **Observed Experience**:
    - Ensure the effort score correlates well with the user's perception of "heavy lifting" (e.g., complex multi-step refactors should have higher effort).

### Proposed Rust Dependencies
- Use existing `src/ui_colors.rs` for subtle coloring.

### Verification Strategy
1. **Behavior**: 
    - Compare scores for a simple "hello" vs. a complex codebase search.
    - Confirm the score is visible but non-intrusive.
2. **Numerical Accuracy**:
    - Verify it matches the internal timing logs in `src/app_chat_trace.rs`.
