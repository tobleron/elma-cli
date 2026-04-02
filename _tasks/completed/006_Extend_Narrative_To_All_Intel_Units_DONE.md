# Task 006: Extend Intel Narrative Module to All Intel Units

## Priority
**P0-2.2 - CRITICAL (PILLAR 2: CONTEXT NARRATIVE)**
**Blocks:** All P0-4 reliability tasks

## Status
**ACTIVE - READY FOR SIGN-OFF** — Remaining stale callers were wired to narrative input and verified

## Renumbering Note
- **Old Number:** Task 047
- **New Number:** Task 006 (per REPRIORITIZED_ROADMAP.md)
- **Reason:** Elevated to P0-2.2 as part of 4 foundational pillars

## Objective

Extend the `intel_narrative` module and its callers so intel units that still receive noisy structured blobs are upgraded to plain-text narrative input.

## Background

Task 046 implemented `intel_narrative.rs` and the critic caller was already migrated. Since then, additional narrative builders were added, but several runtime callers are still using older JSON-heavy request bodies.

**Current reality:** `build_sufficiency_narrative()` and `build_reviewer_narrative()` already exist in `src/intel_narrative.rs`, but the runtime does not consistently use them yet.

## Scope

### Phase 1: Critic (DONE ✅)
- [x] Create `intel_narrative.rs` module
- [x] Implement `build_critic_narrative()`
- [x] Update `request_critic_verdict()` to use narrative format
- [x] Tests passing

### Phase 2: Sufficiency Verifier (TODO)
- [x] Implement `build_sufficiency_narrative()` in `intel_narrative.rs`
- [x] Update `check_execution_sufficiency_once()` in `verification.rs`
- [x] Test sufficiency path

### Phase 3: Reviewers (TODO)
- [x] Implement `build_reviewer_narrative()` in `intel_narrative.rs`
- [x] Update logical/efficiency reviewer request path to use reviewer narrative
- [x] Update risk reviewer request path to use reviewer narrative
- [x] Test logical/efficiency/risk reviewer path

### Phase 4: Other Intel Units (TODO)
- [x] Evaluate remaining JSON-heavy intel inputs
- [x] Migrate evidence mode decision from structured blob to narrative
- [x] Migrate claim/repair judgment helpers to narrative
- [x] Leave text-first pipelines alone when they are already non-JSON-noise

## Implementation Pattern

```rust
// Before (JSON noise):
let input = serde_json::json!({
    "user_message": line,
    "objective": program.objective,
    "speech_act_prior": { "entropy": 0.32, "distribution": [...] },
    "route_prior": { "entropy": 1.06, "distribution": [...] },
    ...
});

// After (plain text narrative):
let narrative = build_critic_narrative(
    &program.objective,
    program,
    step_results,
    attempt,
    max_retries,
);
```

## Expected Benefits

| Benefit | Impact |
|---------|--------|
| **Better model reasoning** | Model focuses on evidence, not technical noise |
| **Consistent format** | All intel units see same narrative structure |
| **Easier debugging** | Plain text is human-readable |
| **Future flexibility** | Can swap to model-based narrative without changing callers |
| **Reduced hallucination** | No entropy numbers to confuse the model |

## Files to Modify

| File | Change |
|------|--------|
| `src/intel_narrative.rs` | Add any missing narrative builders for remaining stale callers |
| `src/verification.rs` | Use narrative for sufficiency and other judgment helpers where applicable |
| `src/orchestration_loop_reviewers.rs` | Use reviewer narrative for logical/efficiency/risk review |
| `src/orchestration_helpers.rs` | Already done for critic |

## Acceptance Criteria

- [x] In-scope intel units no longer receive JSON-heavy program/step blobs when a narrative form exists
- [x] No stale workflow-review callers bypass the narrative module
- [x] Tests passing for all updated units
- [x] Stress tests complete without parse/transport regression

## Dependencies
- ✅ Task 046 (Connection pool fix)
- ✅ Intel narrative module created
- ⏳ This task (extend to all units)

## Notes

**Narrative format example:**
```
OBJECTIVE:
List the files in the _stress_testing/_opencode_for_testing/ directory.

WORKFLOW GENERATED:
Step 1 (shell): Run "find _stress_testing/_opencode_for_testing/ -type f"
  To: List all files in the directory
  Result: Command executed successfully (exit_code=0), output shows 15 files

Step 2 (reply): Present the file list to the user
  To: Answer the user's request
  Result: Response generated with file summary

ATTEMPT: 1 of 2

YOUR TASK:
Does this workflow and its results achieve the objective?
Answer with ONLY: {"status": "ok" or "retry", "reason": "one short sentence"}
```

## Implementation Notes

- Sufficiency verification now uses `build_sufficiency_narrative()` directly.
- Logical, efficiency, and risk reviewer calls now use reviewer narratives instead of JSON program/step blobs.
- Evidence mode, claim checking, and repair semantics guard now use narrative input builders.
- Outcome verification was intentionally left unchanged because it already operates through a text-first pipeline rather than a noisy JSON context blob.

## Verification

- `cargo fmt`
- `cargo build`
- `cargo test` (`143` passed)
- `./run_intention_scenarios.sh` completed successfully
