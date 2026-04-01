# Stress Test S005: High-Intensity Master Planning

## 1. The Test (Prompt)
"Develop a Master Plan to implement a full 'Audit Log' system for the Elma CLI. This log should record every tool call, its parameters, and the model's reasoning content for that turn into a structured JSONL file in `sessions/audit/`. Once planned, implement the first phase: the core trait for auditing and the integration into `orchestration_loop.rs`."

## 2. Debugging Result Understanding
- **Success Criteria**: 
    - The agent selects a `MASTERPLAN` route.
    - It produces a multi-phase decomposition.
    - It successfully implements a new trait in a new file and modifies the main orchestration loop without breaking existing functionality.
- **Common Failure Modes**:
    - Plan is too vague to be actionable.
    - Complexity explosion: The model tries to do everything in one turn and exceeds context.
    - Breaking the `orchestration_loop.rs` due to improper integration of the new trait.

## 3. Bottleneck Detection
- **Decomposition Failure**: The model fails to break the task into "minimum-sufficient" steps.
- **Context Fog**: The large LOC of `orchestration_loop.rs` causes the model to make errors in the `replace` call.

## 4. Resolution & Iteration
- (Iterative refinement to be recorded here during execution)
