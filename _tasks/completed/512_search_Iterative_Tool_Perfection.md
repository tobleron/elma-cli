# Task 512: search - Iterative Tool Perfection

## Objective
Make the 4B model reliably discover, call, and succeed with the `search` tool in every valid scenario, with zero shell fallback when this tool should handle the operation.

## Background
The `search` tool is registered as `RustWrapper` (deferred: No) at `elma-tools/src/tools/search.rs`. It was used 0 times in the last stress-test session — the model defaulted to `shell` for everything instead.

## Stress Test Prompt
```text
Search for all occurrences of the word "TODO" in Rust source files (*.rs), then search for function definitions with regex fn +w+ in src/, then search for unsafe across the codebase excluding target/, then search with zero matches to confirm clean error handling, then combine search results with read to inspect matches.
```

## Suggested Approaches

### Approach A: Prompt Engineering: Regex vs Literal Guidance
Add: 'By default patterns are regex. For literal search, escape special chars.'

### Approach B: Step Decomposition: Search-Then-Read Pipeline
Always search first, then read a few matched files.

### Approach C: Path Scoping: Narrow Searches First
Suggest model narrow by subdirectory to prevent false positives.

### Approach D: Exclusion Patterns: Skip Binary Formats
Ensure search excludes .gguf, .bin, and other binary files.

## Success Criteria
- [ ] The model calls `search` successfully in every scenario from the stress test
- [ ] No shell fallback when `search` should handle the operation
- [ ] All required parameters are populated correctly on every call
- [ ] Error responses are handled gracefully (retry with corrected params, not infinite loops)

## Scenario File
`_testing_prompts/513_search.txt`

## Verification
```bash
cargo build
cargo test
# Then paste the stress test prompt into a running elma-cli session
# and observe whether the model uses the native tool or falls back to shell
```
