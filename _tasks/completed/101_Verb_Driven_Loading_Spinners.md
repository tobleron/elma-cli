# Task 101: Verb-Driven Loading Spinners

## Objective
Enhance user feedback by replacing generic loading spinners with specific action-based verbs like "Thinking...", "Searching...", "Reading...".

## Technical Implementation Plan (Rust)

### Core Requirements
1. **Verb Registry**:
    - Implement a `SpinnerVerb` enum in `src/ui.rs`.
    - Variants: `Thinking`, `Searching`, `Reading`, `Analyzing`, `Executing`, `Synthesizing`.
2. **Animated Spinner Logic**:
    - Implement a `render_spinner(verb: SpinnerVerb, frame: usize)` in `src/ui.rs`.
    - Use a standard sequence of frames: `['\u2801', '\u2802', '\u2804', '\u2808', '\u2810', '\u2820', '\u2840', '\u2880']` (Braille patterns).
3. **Integration**:
    - Hook into `src/intel_units/` and `src/tools/` to pass the correct verb to the UI.
    - Example: `Thinking` when an Intel Unit is running, `Searching` when a search tool is active.
4. **Concurrency**:
    - Run the spinner animation in a background thread or a non-blocking `tokio` task to ensure it stays animated while the agent is waiting for an API response.

### Proposed Rust Dependencies
- `tokio`: To manage the animation loop as a background task.

### Verification Strategy
1. **Behavior**: 
    - Confirm the spinner is fluid and not choppy during long API calls.
    - Confirm the verb correctly updates when the agent transitions from "Searching" to "Analyzing".
2. **Visuals**:
    - Ensure the Braille spinner is high-resolution and high-frame-rate.
