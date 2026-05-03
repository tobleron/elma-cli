# Task 517: git_inspect - Iterative Tool Perfection

## Objective
Make the 4B model reliably discover, call, and succeed with the `git_inspect` tool in every valid scenario, with zero shell fallback when this tool should handle the operation.

## Background
The `git_inspect` tool is registered as `RustWrapper` (deferred: Yes) at `elma-tools/src/tools/git_inspect.rs`. It was used 0 times in the last stress-test session — the model defaulted to `shell` for everything instead.

## Stress Test Prompt
```text
Run git status, then git branch, then git diff --name-only, then git log --oneline -10, then git diff --stat. Run each mode on a specific subdirectory. Run with unknown mode (verify helpful error). Confirm git_inspect is faster and more reliable than shell git commands.
```

## Suggested Approaches

### Approach A: Prompt Engineering: Mode Selection
status=working tree, log=history, branch=branches, diff=content, changed_files=names.

### Approach B: Step Decomposition: Status-Then-Diff-Then-Log
Always call in order: status, diff, log.

### Approach C: Shell-Fallback Guard: Block git in Shell
Block ALL shell commands starting with git. Shell git log persistently times out.

### Approach D: Path-Scoped Inspections
Test path parameter: git_inspect(mode=status, path=src/).

## Success Criteria
- [ ] The model calls `git_inspect` successfully in every scenario from the stress test
- [ ] No shell fallback when `git_inspect` should handle the operation
- [ ] All required parameters are populated correctly on every call
- [ ] Error responses are handled gracefully (retry with corrected params, not infinite loops)
- [ ] The tool appears in `tool_search` results for relevant queries

## Scenario File
`_testing_prompts/529_git_inspect.txt`

## Verification
```bash
cargo build
cargo test
# Then paste the stress test prompt into a running elma-cli session
# and observe whether the model uses the native tool or falls back to shell
```
