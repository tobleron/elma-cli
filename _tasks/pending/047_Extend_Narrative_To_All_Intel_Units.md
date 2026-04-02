# Task: Extend Intel Narrative Module to All Intel Units

## Priority
**P0 - CRITICAL** (Architecture improvement for model reasoning quality)

## Status
**PENDING** — Critic implemented, other units pending

## Objective

Extend the `intel_narrative` module to provide plain-text narrative input for ALL intel units that currently receive noisy JSON blobs.

## Background

Task 046 implemented `intel_narrative.rs` module that transforms structured program/step data into plain-text narratives. The critic now uses this format instead of JSON noise with entropy values and distributions.

**Initial results:** Cleaner input, model can reason about evidence without technical pollution.

## Scope

### Phase 1: Critic (DONE ✅)
- [x] Create `intel_narrative.rs` module
- [x] Implement `build_critic_narrative()`
- [x] Update `request_critic_verdict()` to use narrative format
- [x] Tests passing

### Phase 2: Sufficiency Verifier (TODO)
- [ ] Implement `build_sufficiency_narrative()` in `intel_narrative.rs`
- [ ] Update `run_sufficiency_check()` in `verification.rs`
- [ ] Test sufficiency accuracy

### Phase 3: Reviewers (TODO)
- [ ] Implement `build_reviewer_narrative()` in `intel_narrative.rs`
- [ ] Update `run_staged_reviewers_once()` in `orchestration_loop.rs`
- [ ] Test logical/efficiency/risk reviewer accuracy

### Phase 4: Other Intel Units (TODO)
- [ ] Evidence mode decision
- [ ] Risk review
- [ ] Logical review
- [ ] Efficiency review
- [ ] Outcome verifier

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
| `src/intel_narrative.rs` | Add `build_sufficiency_narrative()`, `build_reviewer_narrative()` |
| `src/verification.rs` | Use narrative for sufficiency |
| `src/orchestration_loop.rs` | Use narrative for reviewers |
| `src/orchestration_helpers.rs` | Already done for critic |

## Acceptance Criteria

- [ ] All intel units use narrative format
- [ ] No JSON entropy/distribution fields in intel input
- [ ] Tests passing for all updated units
- [ ] Stress tests show improved accuracy (fewer false retries)

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
