# Task 515: tool_search - Iterative Tool Perfection

## Objective
Make the 4B model reliably discover, call, and succeed with the `tool_search` tool in every valid scenario, with zero shell fallback when this tool should handle the operation.

## Background
The `tool_search` tool is registered as `RustNative` (deferred: No) at `elma-tools/src/tools/tool_search.rs`. It was used 0 times in the last stress-test session — the model defaulted to `shell` for everything instead.

## Stress Test Prompt
```text
Search for tools related to "git commands status log commit" to discover git_inspect, then search for 'repository structure mapping' to discover repo_map, then search for 'execute python code' to discover run_python, then search for 'node javascript runtime' to discover run_node, then search for 'background job process' to discover job_start, then search for 'fetch URL web content' to discover fetch. Note which queries succeed. Try misspelled queries for fuzzy matching. After each discovery, immediately use the discovered tool.
```

## Suggested Approaches

### Approach A: Prompt Engineering: Query Formulation Strategy
Teach strategy: (1) exact name, (2) synonyms, (3) verb-phrase descriptions.

### Approach B: Step Decomposition: Discover-Then-Use
Every tool_search should be immediately followed by USING the discovered tool.

### Approach C: Query Diversity: 5 Different Queries
Model should try 5+ different queries before concluding tool does not exist.

### Approach D: Discovery Notification Handling
System shows discovery notification on success. Model should acknowledge it.

### Approach E: Fallback: When Nothing Found
Try simpler query with fewer keywords, then fall back to shell.

## Success Criteria
- [ ] The model calls `tool_search` successfully in every scenario from the stress test
- [ ] No shell fallback when `tool_search` should handle the operation
- [ ] All required parameters are populated correctly on every call
- [ ] Error responses are handled gracefully (retry with corrected params, not infinite loops)

## Scenario File
`_testing_prompts/527_tool_search.txt`

## Verification
```bash
cargo build
cargo test
# Then paste the stress test prompt into a running elma-cli session
# and observe whether the model uses the native tool or falls back to shell
```
