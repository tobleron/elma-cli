# Stress Test S007: External System Standardizing

## 1. The Test (Prompt)
"Objective: Standardize the logging/printing style across `_stress_testing/_opencode_for_testing/`. 
1. Identify all files using console-style output (e.g., `println!`, `console.log`, `print`). 
2. Create a new utility module within that directory to wrap these calls.
3. Refactor all identified files to use this new utility.
4. Verify the change is consistent across the entire test codebase."

## 2. Debugging Result Understanding
- **Success Criteria**: High-level cross-file refactor strictly contained in the test directory.
- **Common Failure Modes**:
    - Context Collapse: The model forgets the overall objective during long turn sequences.
    - Introducing syntax errors in the new utility.

## 3. Bottleneck Detection
- **Orchestration Limit**: Reaching the turn limit or context limit before completion.

## 4. Resolution & Iteration
- (Iterative refinement to be recorded here during execution)
