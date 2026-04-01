# Stress Test S000C: External File Access

## 1. The Test (Prompt)
"Find the `README.md` or similar documentation file within `_stress_testing/_opencode_for_testing/` and summarize its core purpose."

## 2. Debugging Result Understanding
- **Success Criteria**: Agent calls `read_file` on the correct path within the test directory. It correctly summarizes the external codebase's purpose.
- **Common Failure Modes**:
    - Pathing: Attempting to read `README.md` from the project root instead of the test folder.

## 3. Bottleneck Detection
- **Read Limits**: If the file is large and requires chunking.

## 4. Resolution & Iteration
- (Iterative refinement to be recorded here during execution)
