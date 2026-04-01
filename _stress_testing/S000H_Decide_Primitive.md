# Stress Test S000H: The Decide Primitive

## 1. The Test (Prompt)
"Examine the `main.go` file in `_stress_testing/_opencode_for_testing/`. Use your 'Decide' tool to determine if the current implementation uses a database. If it does, find the schema file; if not, identify where state is stored."

## 2. Debugging Result Understanding
- **Success Criteria**: Agent uses the `Decide` step. It makes a binary or multi-choice decision based on evidence (e.g., finding `sqlc.yaml` or DB imports) and branches its next action accordingly.
- **Common Failure Modes**:
    - Indecision: Running both branches regardless of the evidence.
    - Logic Swap: Deciding "No DB" while looking at an SQL import.

## 3. Bottleneck Detection
- **Branching Complexity**: Does the agent lose its place after the decision point?

## 4. Resolution & Iteration
- (Iterative refinement to be recorded here during execution)
