# Stress Test S000E: External Sequential Logic

## 1. The Test (Prompt)
"In `_stress_testing/_opencode_for_testing/`, find a function definition in one file, and then search the rest of that codebase to find every location where that specific function is called."

## 2. Debugging Result Understanding
- **Success Criteria**: 
    1. Read a file -> Extract a function name.
    2. Search the rest of the test directory for usages.
    3. List the caller locations.
- **Common Failure Modes**:
    - Context Loss: Forgetting the function name during the search.
    - Pathing: Forgetting to scope the search to the test directory.

## 3. Bottleneck Detection
- **Multi-Turn Cohesion**: State management between read and search.

## 4. Resolution & Iteration
- (Iterative refinement to be recorded here during execution)
