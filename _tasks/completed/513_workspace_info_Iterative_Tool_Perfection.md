# Task 513: workspace_info - Iterative Tool Perfection

## Objective
Make the 4B model reliably discover, call, and succeed with the `workspace_info` tool in every valid scenario, with zero shell fallback when this tool should handle the operation.

## Background
The `workspace_info` tool is registered as `RustNative` (deferred: No) at `elma-tools/src/tools/workspace_info.rs`. It was used 0 times in the last stress-test session — the model defaulted to `shell` for everything instead.

## Stress Test Prompt
```text
Get the workspace overview showing root structure, config files, and project type. Note important files (AGENTS.md, Cargo.toml) and excluded directories. Use this as starting point before exploring with other tools. Verify output includes: workspace root, 5+ top-level entries, and project type detection.
```

## Suggested Approaches

### Approach A: Prompt Engineering: First Tool Convention
Establish: ALWAYS call workspace_info as the very first tool call.

### Approach B: Step Decomposition: Info-Then-Plan
Read workspace_info, formulate plan with tool_search, THEN execute.

### Approach C: Context Re-read on Failure
After tool failure, re-read workspace_info to verify workspace root.

### Approach D: Guidance Snapshot Awareness
Model should reference AGENTS.md rules after reading workspace_info.

## Success Criteria
- [ ] The model calls `workspace_info` successfully in every scenario from the stress test
- [ ] No shell fallback when `workspace_info` should handle the operation
- [ ] All required parameters are populated correctly on every call
- [ ] Error responses are handled gracefully (retry with corrected params, not infinite loops)

## Scenario File
`_testing_prompts/524_workspace_info.txt`

## Verification
```bash
cargo build
cargo test
# Then paste the stress test prompt into a running elma-cli session
# and observe whether the model uses the native tool or falls back to shell
```
