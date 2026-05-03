# Task 525: edit - Iterative Tool Perfection

## Objective
Make the 4B model reliably discover, call, and succeed with the `edit` tool in every valid scenario, with zero shell fallback when this tool should handle the operation.

## Background
The `edit` tool is registered as `RustNative` (deferred: No) at `elma-tools/src/tools/edit.rs`. It was used 0 times in the last stress-test session — the model defaulted to `shell` for everything instead.

## Stress Test Prompt
```text
Open the file src/main.rs, add a new struct AppConfig with fields for host: String and port: u16 right after the Args struct definition. Then replace the string "localhost" with "0.0.0.0" in the fn main function. After that, change the line println!(Hello) to println!(Hello, world!) and verify the edit was applied correctly by reading the changed sections. Also demonstrate removing an entire function definition that is no longer needed.
```

## Suggested Approaches

### Approach A: Prompt Engineering: Required-Param Anchoring
Embed required parameters (path, old_string, new_string) directly in the prompt task description in a bullet checklist format. This reduces the chance the model omits parameters.

### Approach B: Step Decomposition: Read-Edit-Read Loop
Train the model to always: read the target section first (to get exact old_string context), then edit, then read the changed lines back to confirm.

### Approach C: Shell-Fallback Guard: Blocking shell sed
Block shell calls containing sed, awk, or perl -i to force the model to use edit.

### Approach D: Token Budget Reduction: Force Multi-Edit
Set max_tokens very low (512) so the model uses incremental edit calls.

## Success Criteria
- [ ] The model calls `edit` successfully in every scenario from the stress test
- [ ] No shell fallback when `edit` should handle the operation
- [ ] All required parameters are populated correctly on every call
- [ ] Error responses are handled gracefully (retry with corrected params, not infinite loops)

## Scenario File
`_testing_prompts/506_edit.txt`

## Verification
```bash
cargo build
cargo test
# Then paste the stress test prompt into a running elma-cli session
# and observe whether the model uses the native tool or falls back to shell
```
