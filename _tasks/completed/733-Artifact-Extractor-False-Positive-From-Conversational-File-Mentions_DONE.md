# Task 733: Artifact Extractor False Positive From Conversational File Mentions

## Type

Bug

## Severity

High

## Scope

System-wide

## Session Evidence

Session `s_1778151871_491015000`, Turn 2:

User message: *"You gave me this opinion after reading how many docs? It seems you have only read README.md"*

Trace log:
```
trace: artifact_tracking: required=README.md,docs/README.md
```

Final answer (`0019_final_answer.md`):
```
Completed the requested artifact work.
Created or updated:
- `README.md`
- `docs/README.md`
```

The user was **not** requesting that `README.md` or `docs/README.md` be created. They were questioning whether README.md had been read as part of the investigation. The word `README.md` appeared in the user's message in a conversational context (accusation/question), not as a file creation request.

`extract_required_artifacts_from_request` (in `artifact_verifier.rs`) scanned the user message, found `README.md` as a path-like string, and registered it as a required artifact deliverable. The final answer then became "Completed the requested artifact work" — complete semantic drift from the actual user intent.

The user's original question ("how many docs did you read?") was never answered. Instead they received a fabricated "artifact completed" answer.

## Problem

`extract_required_artifacts_from_request` at `artifact_verifier.rs:440` treats any file path found in the user's message as a potential required output artifact. It checks only for the presence of file-creation keywords (`create`, `write`, `save`, etc.) in the overall request, not in the specific line containing the file path. This means:

- *"It seems you have only read README.md"* → extracts `README.md` as a required artifact because the sentence also contained `read` (which is NOT a creation verb, but the function checks for `read` as a creation keyword at line 457: `lower_line.contains("read")`)

Wait — looking at the code more carefully:

```rust
fn extract_required_artifacts_from_request(user_request: &str) -> Vec<String> {
    ...
    let is_output_request = {
        let lower_line = trimmed.to_lowercase();
        lower_line.contains("create")
            || lower_line.contains("write")
            || lower_line.contains("save ")
            || lower_line.contains("output ")
            || lower_line.contains("generate")
            || lower_line.contains("produce")
            || lower_line.ends_with(".md")
            || lower_line.ends_with(".txt")
            ...
    };
```

The function checks `lower_line.contains("read")` — no, wait, it's NOT in the list. Let me re-read...

Actually the issue is `lower_line.ends_with(".md")` — the user's line *"It seems you have only read README.md"* ends with `.md`. So ANY LINE ending with `.md` is treated as an output request. This is the bug: `ends_with(".md")` is a gross over-match.

## Root Cause Hypothesis

Confirmed: `extract_required_artifacts_from_request` at line 461 has `lower_line.ends_with(".md")` and `.txt`, `.json`, `.rs`, `.toml` as file-creation heuristics. Any user message line ending with these extensions triggers artifact extraction for any path-like words found in the line, regardless of whether the user is actually requesting file creation.

The fix: remove `ends_with` extension checks from `is_output_request`. A file extension alone is not a file creation request. Only explicit creation verbs (`create`, `write`, `save`, `generate`, `produce`, `output`) should trigger artifact extraction.

## Proposed Solution

In `src/artifact_verifier.rs`, `extract_required_artifacts_from_request`:

1. Remove the extension-based heuristics:
   ```rust
   || lower_line.ends_with(".md")
   || lower_line.ends_with(".txt")
   || lower_line.ends_with(".json")
   || lower_line.ends_with(".rs")
   || lower_line.ends_with(".toml")
   ```

2. Keep only the explicit creation-verb heuristics:
   ```rust
   lower_line.contains("create")
       || lower_line.contains("write")
       || lower_line.contains("save ")
       || lower_line.contains("output ")
       || lower_line.contains("generate")
       || lower_line.contains("produce")
   ```

3. Add a guard against `read` being treated as a creation context: if the line contains `read` without a creation verb, skip it even if it has a path. The current `contains("read")` check already doesn't exist in the list, but `ends_with(".md")` was catching it.

## Acceptance Criteria

- [ ] User message *"It seems you have only read README.md"* does NOT register `README.md` as a required artifact.
- [ ] User message *"Create a report in project_tmp/security_report.md"* still correctly extracts `project_tmp/security_report.md`.
- [ ] User message *"Write the output to project_tmp/findings.txt"* still correctly extracts `project_tmp/findings.txt`.
- [ ] `cargo test -q artifact_verifier` passes all existing tests.
- [ ] Replaying Turn 2 of `s_1778151871_491015000` does not register false artifacts.

## Verification Plan

- Unit test: *"It seems you have only read README.md"* → no artifacts extracted.
- Unit test: *"Create project_tmp/report.md and save findings.json"* → both extracted.
- Unit test: *"Check docs/README.md for instructions"* → no artifacts extracted (it's a read context, not create).
- Existing extraction tests pass or are updated to reflect the narrower heuristic.

## Dependencies

None.

## Notes

The original `ends_with` heuristic was likely added as a catch-all for messages like "I need a project_tmp/report.md" where no explicit verb is present but the intent is clear. The fix removes that heuristic because it causes more harm (false positives destroying semantic continuity) than good. Users who want files created should use explicit verbs.
