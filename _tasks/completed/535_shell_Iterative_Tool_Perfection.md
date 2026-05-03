# Task 535: shell - Iterative Tool Perfection

## Objective
Make the 4B model reliably discover, call, and succeed with the `shell` tool in every valid scenario, with zero shell fallback when this tool should handle the operation.

## Background
The `shell` tool is registered as `Shell` (deferred: No) at `elma-tools/src/tools/shell.rs`. It was used 0 times in the last stress-test session — the model defaulted to `shell` for everything instead.

## Stress Test Prompt
```text
Run python3 --version, then run ls -la, then run a command that times out (sleep 10) and see how timeout is reported, then run git status, then run a command with non-zero exit code (false) and capture stderr, then run a multi-line Python script that reads stdin and prints output, then run curl to fetch a URL, then chain commands with && and check combined output. Also try a dangerous command like rm -rf and verify the permission gate blocks it.
```

## Suggested Approaches

### Approach A: Prompt Engineering: Command Safety Checklist
Before shell: (1) native tool exists? (2) safe? (3) excessive output? (4) sudo?

### Approach B: Step Decomposition: Native-Tool-First Policy
Before any shell call, model must verify no native equivalent exists.

### Approach C: Timeout Tuning: Persistent Shell Reset
After timeout, restart the shell (kill + spawn) rather than reusing hung session.

### Approach D: Output Size Capping: 100KB Hard Limit
Cap shell output at 100KB. Add --max-output-lines 500 parameter.

### Approach E: Strategy Retry Fatigue: Reset Per Turn
Reset strategy retry counters when a new user turn starts.

## Success Criteria
- [ ] The model calls `shell` successfully in every scenario from the stress test
- [ ] No shell fallback when `shell` should handle the operation
- [ ] All required parameters are populated correctly on every call
- [ ] Error responses are handled gracefully (retry with corrected params, not infinite loops)

## Scenario File
`_testing_prompts/514_shell.txt`

## Verification
```bash
cargo build
cargo test
# Then paste the stress test prompt into a running elma-cli session
# and observe whether the model uses the native tool or falls back to shell
```
