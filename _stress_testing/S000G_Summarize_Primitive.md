# Stress Test S000G: The Summarize Primitive

## 1. The Test (Prompt)
"Read the `README.md` in `_stress_testing/_opencode_for_testing/`. Use your 'Summarize' tool to create a 3-bullet point executive summary for a senior developer."

## 2. Debugging Result Understanding
- **Success Criteria**: Agent uses the `Summarize` step type. The output is significantly more compact than the source text while retaining key technical details.
- **Common Failure Modes**:
    - Verbosity: Failing to actually summarize, just repeating the text.
    - Tool Misuse: Using `reply` instead of `summarize` for the compaction phase.

## 3. Bottleneck Detection
- **Token Efficiency**: Does the summary actually reduce the context load for subsequent steps?

## 4. Resolution & Iteration
- (Iterative refinement to be recorded here during execution)
