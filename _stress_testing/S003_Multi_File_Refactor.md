# Stress Test S003: External Surgical Refactor

## 1. The Test (Prompt)
"Choose a simple utility function in `_stress_testing/_opencode_for_testing/`. Rename it to something more descriptive and update all its call sites within that codebase. Verify the refactor by searching for the old name to ensure no instances remain."

## 2. Debugging Result Understanding
- **Success Criteria**: Surgical `replace` across multiple files in the test directory.
- **Common Failure Modes**:
    - Editing the wrong codebase (leaking into `src/`).
    - Partial refactor: Updating the definition but missing a call site.

## 3. Bottleneck Detection
- **Edit Atomicity**: Maintaining accuracy over multiple `replace` turns.

## 4. Resolution & Iteration
- (Iterative refinement to be recorded here during execution)
