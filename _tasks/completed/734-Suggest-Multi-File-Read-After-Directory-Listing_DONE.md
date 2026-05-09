# Task 734: Suggest Multi-File Read After Directory Listing

## Type

Model Robustness / Efficiency

## Severity

Medium

## Scope

System-wide

## Session Evidence

Session `s_1778151871_491015000`, all three task turns:

Each turn follows the same pattern:
1. `ls docs` → 55 items
2. `read docs/README.md` → single file read
3. 5 iterations wasted on duplicate read attempts

The model never uses `read` with `paths` (plural, array argument) to batch-read multiple files. In Turn 3, the model explicitly identified the files it SHOULD read in its thinking:

```
Key Documentation Files Identified:
- ARCHITECTURE.md (48KB)
- TOOL_CALLING_PIPELINE.md (17KB)  
- EVIDENCE_LEDGER.md (8KB)
- DEVELOPMENT.md (23KB)
```

...but it generated only a single `read docs/README.md` tool call. The model's verbalized plan and its actual actions are disconnected. By the time it gets to iteration 3+, the context is polluted with duplicate-suppression messages and it can no longer execute its plan.

## Problem

The small (4B) model can identify what files to read but only emits one `read` at a time. After the first read succeeds, the duplicate gate blocks re-reads, and the model can't reassemble its plan to read the remaining files. A single multi-file `read` with `paths: ["ARCHITECTURE.md", "TOOL_CALLING_PIPELINE.md", ...]` would be 5-10× more efficient and avoid the duplicate-read death spiral entirely.

The `read` tool schema accepts both `path` (single) and `paths` (array), but the model defaults to single `path` and the system doesn't suggest the more efficient option.

## Root Cause Hypothesis

Confirmed: The model is never prompted or nudged to use multi-file `paths`. The `read` tool description says "It can read multiple files at once efficiently" but the model ignores this capability under cognitive load. A system-level hint after directory listing would bridge this gap.

## Proposed Solution

After the model successfully lists a directory (`ls`, `glob`) and then calls `read` with a single file, inject a compact hint suggesting multi-file reads for subsequent reads:

```
Hint: Use 'read' with 'paths' (plural, array) to read multiple files at once.
Example: read paths=["docs/ARCHITECTURE.md", "docs/TOOL_CALLING_PIPELINE.md", "docs/EVIDENCE_LEDGER.md"]
```

Implementation plan:

- `src/tool_loop.rs`: In the tool result processing section (after a successful `read` with single path), check if the most recent tool call before this was an `ls` or `glob` that returned multiple files.
  - If so, inject the multi-file hint as a system message.
  - Only do this once per turn (the hint should be injected after the first single-path read).
  - Count the number of files in the `ls` output — only inject if there are > 3 files (small directories don't benefit from batching).

- `src/tool_repair.rs`: Add `count_files_in_ls_output(content: &str) -> usize` helper.

## Acceptance Criteria

- [ ] After `ls docs` (55 items) followed by single `read docs/README.md`, a multi-file hint is injected.
- [ ] The hint includes concrete example paths from the `ls` output.
- [ ] After `ls` of a small directory (items <= 3), no hint is injected.
- [ ] The hint is injected at most once per turn.
- [ ] The hint is suppressed if the model already used multi-file `paths` in the first read.
- [ ] Replaying Turn 1 of `s_1778151871_491015000` shows the model using multi-file read after the hint.

## Verification Plan

- Unit test: `count_files_in_ls_output` with sample ls output → returns 55.
- Unit test: Simulated turn with `ls` → single `read` → verify hint injected.
- Unit test: Small directory `ls` → single `read` → no hint injected.
- Manual replay of Turn 1 with hint → model reads multiple files in one call.

## Dependencies

- Task 732 (strategy shift on stuck read) — these two tasks complement each other: 734 helps prevent getting stuck, 732 handles the case when it already happened.

## Notes

The core insight: the model KNOWS what to do (it listed the files to read in its thinking) but can't execute it with single-file reads. The system should bridge this execution gap by suggesting the efficient batching path.
