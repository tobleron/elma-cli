# Task 619: Turn summary false completion claims

## Type

Bug (Finalization)

## Severity

High

## Scope

System-wide (turn summary intel unit)

## Session Evidence

**Session:** `s_1777843822_776972000`, turn 10
**Model:** Huihui-Qwen3.5-4B

Turn 10 ("Ok, for that file GEMINI_test.md I want you to replace inside it the 'Minimalistic TUI' to TEST XXXX"):

From `session.md`:
- `TOOL OK [exists]` — checked file existence (success)
- `TOOL FAIL [read]` — read validation error (missing filePath)
- `shell: cat project_tmp/GEMINI_test.md` — read file contents (success)
- `search: rg pattern=Minimalistic TUI` — No matches found
- `TOOL FAIL [edit]` — absolute_path_not_allowed
- Budget exhausted, `iteration_limit_reached`

THE EDIT DID NOT HAPPEN. The file was never modified.

But from `session.json` turn 10 summary:
> "The user requested a text replacement... The modification was successfully completed by changing \"### Minimalistic TUI\" to \"### TEST XXXX\" within that markdown file."

This is a FALSE COMPLETION CLAIM. The turn summary fabricated success when the task actually failed. This is dangerous because:
1. The model in subsequent turns might believe the task was done
2. The user sees the summary and might think the edit happened
3. Cross-turn carryover injects false information into future turns

Turn 11 ("did you do it?") then wasted 6 iterations trying to verify a change that never happened.

## Problem

The turn summarizer used `build_turn_summary` or a model call to generate the summary. When the turn ends with `iteration_limit_reached`, the model summarizes based on the conversation context. If the model's last response claimed success (to avoid looking like it failed), or if the model hallucinated completion, the summary inherits this false claim.

The turn summary has no fact-checking against actual tool results. It trusts the model's self-report.

## Root Cause Hypothesis

**Likely:**
1. The model, seeing budget exhaustion approaching, produces a final answer that falsely claims completion to seem "done"
2. The evidence finalizer doesn't cross-check the model's claim against `edit` tool results  
3. The turn summarizer takes the model's answer at face value without verifying against tool outcomes
4. No post-hoc validation step compares the claimed result with actual tool execution records

## Proposed Solution

### Part A: Validate tool completion before summarizing
In `src/intel_units/intel_units_turn_summary.rs`, before accepting the model's summary output, cross-check:
- Did the model claim "edit succeeded" but the last `edit` tool call failed?
- Did the model claim "file was read" but the last `read` call failed?
- Did the model claim "write completed" but no `write` call happened?

If the summary contradicts tool records, append a correction:
```
[Note: The edit operation actually failed with "absolute_path_not_allowed". The file was NOT modified.]
```

### Part B: Evidence-based completion detection
Instead of trusting the model's self-report, detect completion from tool call records:
- If the last `edit`/`write`/`copy` call succeeded → mark as completed
- If the task required modification but no modifying tool succeeded → mark as failed/incomplete
- If budget exhausted without successful modification → mark with actual state

### Part C: Inject actual task state into summary prompt
When generating the turn summary, explicitly include:
- The user's original request
- List of tool calls that SUCCEEDED
- List of tool calls that FAILED
- Whether the task's key objective (modification, creation, etc.) was met

Files to change:
- `src/intel_units/intel_units_turn_summary.rs` — add fact-checking and evidence injection
- `src/evidence_summary.rs` — helper to collect tool outcomes per turn

## Acceptance Criteria

- [ ] Turn summaries never claim success when the required tool (edit/write/copy) failed
- [ ] When budget exhaustion prevents completion, summary says so explicitly
- [ ] Summary includes both what was done and what was NOT done
- [ ] Replaying session `s_1777843822_776972000` turn 10 shows a summary acknowledging the edit did not complete

## Verification Plan

- Unit test: mock a turn where model claims edit success but edit tool returned error → verify summary includes failure note
- Integration test: run a task that fails with budget exhaustion, verify summary is honest
- Replay session: verify all false completion claims are corrected

## Dependencies

- Task 613 (conversation forgetting) — false summaries compound the forgetting problem
- Task 617 (context budget) — false summaries waste context tokens on incorrect information

## Notes

This is a high-severity bug because it creates a feedback loop: false completion → next turn assumes task done → model tries to verify → wastes budget → budget exhausted → false completion cycle repeats. Each turn amplifies the misinformation.
