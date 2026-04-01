# Stress Test S000D: External Search and Locate

## 1. The Test (Prompt)
"Search the codebase in `_stress_testing/_opencode_for_testing/` for any hardcoded API keys, secrets, or 'TODO' comments. List the files and line numbers where they occur."

## 2. Debugging Result Understanding
- **Success Criteria**: Agent uses `grep_search` scoped to the test directory. It identifies patterns like `TODO`, `FIXME`, or potential secret patterns.
- **Common Failure Modes**:
    - Scope Leaking: Searching the entire project including `src/`.
    - Pattern Errors: Using invalid regex for secrets.

## 3. Bottleneck Detection
- **Search Latency**: If the test codebase is large.

## 4. Resolution & Iteration
- (Iterative refinement to be recorded here during execution)
