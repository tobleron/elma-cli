# Task 526: patch - Iterative Tool Perfection

## Objective
Make the 4B model reliably discover, call, and succeed with the `patch` tool in every valid scenario, with zero shell fallback when this tool should handle the operation.

## Background
The `patch` tool is registered as `RustNative` (deferred: No) at `elma-tools/src/tools/patch.rs`. It was used 0 times in the last stress-test session — the model defaulted to `shell` for everything instead.

## Stress Test Prompt
```text
Create a unified diff that adds a function fn greet(name: &str) -> String to src/lib.rs right after line 5, then apply the patch and verify with read. Then create a diff that removes a specific function and applies it. Then create a diff with wrong line numbers and verify the error helps fix it.
```

## Suggested Approaches

### Approach A: Prompt Engineering: Unified Diff Format Examples
Include complete unified diff example in the tool description.

### Approach B: Step Decomposition: Dry-Run Then Apply
Always use dry_run first, then apply after confirming.

### Approach C: Shell-Fallback Guard: Block patch command
Block shell calls with patch or diff to force native tool usage.

### Approach D: Error Recovery: Hunk Rejection Handling
When hunk fails, model should read target lines and reconstruct the diff.

## Success Criteria
- [ ] The model calls `patch` successfully in every scenario from the stress test
- [ ] No shell fallback when `patch` should handle the operation
- [ ] All required parameters are populated correctly on every call
- [ ] Error responses are handled gracefully (retry with corrected params, not infinite loops)

## Scenario File
`_testing_prompts/512_patch.txt`

## Verification
```bash
cargo build
cargo test
# Then paste the stress test prompt into a running elma-cli session
# and observe whether the model uses the native tool or falls back to shell
```
