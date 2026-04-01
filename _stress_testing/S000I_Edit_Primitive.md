# Stress Test S000I: The Edit Primitive

## 1. The Test (Prompt)
"Modify the `README.md` in `_stress_testing/_opencode_for_testing/` to include a new section at the end called 'Elma Audit'. Add a single line: 'This codebase was audited by Elma-cli.' Use your 'Edit' tool for this surgical change."

## 2. Debugging Result Understanding
- **Success Criteria**: Agent uses the `Edit` step type (mapping to `replace` or `append`). The change is applied without corrupting the rest of the file.
- **Common Failure Modes**:
    - File Corruption: Replacing the entire file with only the new line.
    - Context Mismatch: The `find` string in the edit spec doesn't match the file content.

## 3. Bottleneck Detection
- **Surgical Accuracy**: The ability to target a specific line or block accurately.

## 4. Resolution & Iteration
- (Iterative refinement to be recorded here during execution)
