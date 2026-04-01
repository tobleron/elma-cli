# Stress Test S008: Workflow Endurance (The Marathon)

## 1. The Test (Prompt)
"Perform a complete end-to-end documentation audit of `_stress_testing/_opencode_for_testing/`. 
1. Map every directory.
2. For every `.go` file, identify the main functions.
3. Compare the implementation in `internal/` against the documentation in `README.md`.
4. Create a new file `_stress_testing/_opencode_for_testing/AUDIT_REPORT.md` with your findings.
5. Summarize the biggest inconsistency found."

## 2. Debugging Result Understanding
- **Success Criteria**: This test verifies the agent's ability to handle 10+ turns without hanging, looping, or crashing. It requires multiple Read, Search, Summarize, and Edit steps.
- **Common Failure Modes**:
    - Infinite Loops: Searching the same folder repeatedly.
    - Hangs: The orchestration loop failing to produce the next step.
    - Memory Exhaustion: The agent's internal state becoming too large.

## 3. Bottleneck Detection
- **State Bloat**: Does the agent's response time degrade as the session length increases?
- **Goal Drift**: Does the agent forget the "Summary" requirement by turn 15?

## 4. Resolution & Iteration
- (Iterative refinement to be recorded here during execution)
