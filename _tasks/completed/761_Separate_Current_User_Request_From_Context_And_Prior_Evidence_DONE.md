# Task 761: Separate Current User Request From Context And Prior Evidence

## Type

Architecture / Context Hygiene / Semantic Continuity

## Severity

Critical

## Scope

Tool-calling pipeline, turn context packet, artifact extraction, finalization

## Problem

The latest session shows prior evidence being merged into the current user request. After the user asked `root path has no AGENTS.md ?`, the runtime traced `artifact_tracking: required=AGENTS.md` and returned artifact-completion text instead of answering the existence question. Later, after the user asked to verify completed tasks, stale `_testing_prompts` artifact requirements leaked into the turn.

Relevant code:

- `src/orchestration_core.rs` appends `[Previously gathered in a prior attempt]` directly to the user line before calling the tool loop.
- `src/tool_loop.rs` then treats that combined string as `original_user_request` and extracts artifacts from it.

## Root Cause

The current architecture does not maintain a hard boundary between the raw current user request, prior evidence, runtime hints, and system guidance. Downstream components read the combined prompt and cannot know which text came from the user.

## Proposed Solution

- Introduce a `CurrentTurnContext` or equivalent struct with separate fields:
  - `raw_user_request`
  - `current_objective`
  - `prior_relevant_evidence`
  - `runtime_recovery_hints`
  - `system_guidance`
- Pass this structured context into tool loop, artifact extraction, scope extraction, and finalization.
- Artifact and scope extraction must read only `raw_user_request` or explicit current-turn objective fields.
- Prior evidence may guide tool strategy, but must never create required artifacts or mutate the current objective.
- Persist the rendered packet in the session for debugging.

## Acceptance Criteria

- [ ] Prior evidence is no longer concatenated into the current user request.
- [ ] Artifact extraction ignores prior evidence and runtime hints.
- [ ] The `root path has no AGENTS.md ?` prompt produces an existence-oriented answer, not artifact completion.
- [ ] The completed-task verification prompt cannot inherit `_testing_prompts` deliverables from the previous turn.
- [ ] Trace artifacts show the raw user request separately from injected context.

## Verification Plan

- Unit test artifact extraction with a raw request plus prior evidence that mentions file paths.
- Replay the latest session prompt sequence and assert no stale artifact requirements are created.
- Inspect `trace_debug.log` and `session.json` to confirm separated context fields.

## Dependencies

Do before Tasks 762, 766, and 768.

## Notes

Do not solve this by adding prompt warnings. This is a data-flow boundary problem.

