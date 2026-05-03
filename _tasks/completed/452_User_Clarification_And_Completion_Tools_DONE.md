# Task 452: User Clarification And Completion Tools

**Status:** Pending
**Priority:** LOW
**Estimated effort:** 2-3 days
**Dependencies:** Task 380, Task 388
**References:** source-agent parity: ask-followup and attempt-completion behavior

## Objective

Add structured clarification and completion behavior so Elma can ask for missing information or finish tasks cleanly without pretending uncertainty is resolved.

## Implementation Plan

1. Add a lightweight `clarification_needed` intel unit with strict JSON:
   - `needed`
   - `question`
   - `reason`
2. Add a user-facing clarification tool/path that pauses execution and records the missing decision.
3. Add a completion check that verifies the original objective was satisfied before final response.
4. Integrate with semantic continuity and work graph state.
5. Keep terminal output plain text and direct.

## Verification

```bash
cargo test clarification
cargo test continuity
cargo test orchestration
cargo build
```

## Done Criteria

- Elma asks concise questions when required information is genuinely missing.
- Completion is checked against the original objective.
- Clarification does not become a keyword-triggered bypass.

