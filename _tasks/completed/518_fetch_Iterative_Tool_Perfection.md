# Task 518: fetch - Iterative Tool Perfection

## Objective
Make the 4B model reliably discover, call, and succeed with the `fetch` tool in every valid scenario, with zero shell fallback when this tool should handle the operation.

## Background
The `fetch` tool is registered as `Network` (deferred: Yes) at `elma-tools/src/tools/fetch.rs`. It was used 0 times in the last stress-test session — the model defaulted to `shell` for everything instead.

## Stress Test Prompt
```text
Fetch https://raw.githubusercontent.com/anthropics/anthropic-cookbook/main/README.md in markdown format. Fetch same URL in text format (compare outputs). Fetch in html format. Fetch with timeout=5. Attempt file:///etc/passwd (rejected). Attempt ftp://example.com (rejected). Fetch non-existent URL (HTTP error reported). Fetch redirecting URL (http://example.com) (confirm followed).
```

## Suggested Approaches

### Approach A: Prompt Engineering: URL Validation
ONLY http:// and https:// URLs. File, ftp, others rejected.

### Approach B: Step Decomposition: Fetch-Then-Process
After fetch, process content: summarize or extract key data.

### Approach C: Format Selection: When to Use Each
markdown for HTML, text for raw, html for markup. Default markdown.

### Approach D: Timeout Awareness
If timeout, retry with longer timeout. Max 120s.

### Approach E: Shell-Fallback Guard: Block curl and wget
Block shell commands containing curl or wget.

## Success Criteria
- [ ] The model calls `fetch` successfully in every scenario from the stress test
- [ ] No shell fallback when `fetch` should handle the operation
- [ ] All required parameters are populated correctly on every call
- [ ] Error responses are handled gracefully (retry with corrected params, not infinite loops)
- [ ] The tool appears in `tool_search` results for relevant queries

## Scenario File
`_testing_prompts/536_fetch.txt`

## Verification
```bash
cargo build
cargo test
# Then paste the stress test prompt into a running elma-cli session
# and observe whether the model uses the native tool or falls back to shell
```
