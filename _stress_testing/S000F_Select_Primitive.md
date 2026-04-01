# Stress Test S000F: The Select Primitive

## 1. The Test (Prompt)
"In `_stress_testing/_opencode_for_testing/`, identify three potential files that could be the main application logic. Use your internal 'Select' tool to choose the most likely candidate and explain your reasoning."

## 2. Debugging Result Understanding
- **Success Criteria**: Agent uses the `Select` step type. It presents options and successfully chooses one based on file naming or content hints (e.g., `main.go` vs `sqlc.yaml`).
- **Common Failure Modes**:
    - Skipping Select: Just picking one without using the formal `select` step.
    - Hallucination: Selecting a file that doesn't exist.

## 3. Bottleneck Detection
- **Selection Logic**: If the model provides poor criteria for selection.
- **Workflow Interruption**: If the `select` step causes a hang in the orchestration loop.

## 4. Resolution & Iteration
- (Iterative refinement to be recorded here during execution)
