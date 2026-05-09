# Task 732: Inject Strategy Shift When Model Reads Same File Repeatedly

## Type

Bug / Model Robustness

## Severity

High

## Scope

System-wide

## Session Evidence

Session `s_1778151871_491015000`, Turns 1, 2, 3 — consistent pattern across all three:

```
> ls: docs/  (55 item(s))
    001-fetch-sandboxing.md, 002-sub-agent-delegation.md, ..., ARCHITECTURE.md (48KB),
    TOOL_CALLING_PIPELINE.md (17KB), DEVELOPMENT.md (23KB), ...

> read: docs/README.md  ✓ (success)
> (duplicate skipped: read docs/README.md)  × 5 times
> [STAGNATION] stagnation run 5
> stop reason: respond_abuse
```

Trace log for Turn 2:
```
trace: tool_loop: iteration 2/9 → read docs/README.md (success)
trace: tool_loop: duplicate skipped (already succeeded) signal=read: (×5 iterations)
trace: tool_loop: stopping reason=respond_abuse
```

The model listed 55 docs, read exactly one file (README.md), then spent 5 iterations trying to re-read it before giving up. It never read `ARCHITECTURE.md`, `TOOL_CALLING_PIPELINE.md`, `DEVELOPMENT.md`, or any other file. This pattern repeated in Turns 1, 2, and 3 identically.

## Problem

After a successful file read, the duplicate gate blocks re-reads of the same file. But the duplicate gate gives only a generic message: "Already completed earlier — same result: ...". The model doesn't understand it should choose a DIFFERENT file. With small models (4B), the model gets confused by the rejection, tries the same read again, and wastes 5 iterations before hitting stagnation.

The system knows what OTHER files are available (from the earlier `ls` output in the conversation history), but it doesn't surface those candidates to the model when it's stuck.

## Root Cause Hypothesis

Confirmed: After blocking a duplicate read, the system injects a message saying "Already completed earlier — same result" but does not suggest alternative files. The model doesn't have enough context to pick a different file from the 55-item directory listing. For small models, this is a high-cognitive-load decision.

## Proposed Solution

After 2+ consecutive duplicate read suppressions in the same turn, inject a strategy-shift hint that:

1. Extracts alternative file paths from the most recent `ls`, `glob`, or `search` output in the conversation
2. Lists 3-5 specific candidate files as suggestions
3. Suggests using `read` with `paths` (plural, array) to batch-read them

Implementation plan:

- `src/tool_loop.rs`: In the duplicate gate section (around line 2133), after a duplicate read is suppressed, check if this is the 2nd+ consecutive read duplicate.
  - If so, scan recent messages for `ls` output and extract file paths.
  - Build a hint like: "You've already read `docs/README.md`. Try reading OTHER files from the listing: `ARCHITECTURE.md`, `TOOL_CALLING_PIPELINE.md`, `DEVELOPMENT.md`. Use read with multiple paths: `paths: [\"docs/ARCHITECTURE.md\", ...]`"
  - Push this as a system message to the model.

- `src/tool_repair.rs`: Add `extract_file_paths_from_ls_output(content: &str, exclude: &HashSet<String>, max: usize) -> Vec<String>` to parse file paths from `ls`-style output, excluding already-read files.

## Acceptance Criteria

- [ ] After 2+ duplicate read suppressions in a turn, a strategy-shift hint is injected with alternative file paths.
- [ ] The hint lists files from the most recent `ls`/`glob`/`search` output.
- [ ] Already-read files are excluded from the suggestion list.
- [ ] The hint suggests using multi-file `paths` for batch reading.
- [ ] The hint is injected at most once per turn (not on every subsequent duplicate).
- [ ] Replaying Turn 2 of `s_1778151871_491015000` shows the model trying to read a different file after the hint.

## Verification Plan

- Unit test: `extract_file_paths_from_ls_output` with sample `ls docs` output → correctly extracts filenames.
- Unit test: Sample turn with read duplicate × 2 → strategy shift injected on 2nd duplicate.
- Manual replay of Turn 1 with hint injection → model reads files beyond README.md.

## Dependencies

None.

## Notes

The broader solution would be to have the model use `paths` (plural) to batch-read files proactively, but this task focuses on the minimum fix: when stuck, show the model what other files are available.
