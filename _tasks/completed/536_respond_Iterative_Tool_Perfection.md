# Task 536: respond - Iterative Tool Perfection

## Objective
Make the 4B model reliably discover, call, and succeed with the `respond` tool in every valid scenario, with zero shell fallback when this tool should handle the operation.

## Background
The `respond` tool is registered as `RustNative` (deferred: No) at `elma-tools/src/tools/respond.rs`. It was used 0 times in the last stress-test session — the model defaulted to `shell` for everything instead.

## Stress Test Prompt
```text
Send a formatted response to the user with a summary of the conversation, including a code block, bullet list, and comparison table. Then send a second response acknowledging the user question with next steps. Verify content is visible in TUI and markdown renders correctly. Test empty content (shows nothing) and very long content (>2000 chars, verify truncation).
```

## Suggested Approaches

### Approach A: Prompt Engineering: Content-Only Reminder
Remind model that respond only takes content (and optionally format).

### Approach B: Content Truncation: Safety Limit
Verify model never sends more than 4000 tokens in a single respond.

### Approach C: Streaming Awareness
Model should know respond content is streamed to TUI in real-time.

### Approach D: Format Parameter Testing
Test markdown, text, json format parameter.

## Success Criteria
- [ ] The model calls `respond` successfully in every scenario from the stress test
- [ ] No shell fallback when `respond` should handle the operation
- [ ] All required parameters are populated correctly on every call
- [ ] Error responses are handled gracefully (retry with corrected params, not infinite loops)

## Scenario File
`_testing_prompts/515_respond.txt`

## Verification
```bash
cargo build
cargo test
# Then paste the stress test prompt into a running elma-cli session
# and observe whether the model uses the native tool or falls back to shell
```
