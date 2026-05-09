# Task 749: Remove Keyword-Based Goal Drift Detection From guardrails.rs

## Type

Architecture / Rule 1 Violation

## Severity

High

## Scope

Guardrails, goal drift detection, program validation

## Problem

`src/guardrails.rs` implements goal drift detection using hardcoded keyword lists to infer user intent from the objective string:

```rust
fn check_step_goal_mismatch(objective: &str, program: &Program) -> Option<String> {
    let objective_lower = objective.to_lowercase();

    let action_keywords = [
        "delete", "remove", "add", "create", "update", "fix", "run", "execute",
    ];
    let is_action_goal = action_keywords.iter().any(|kw| objective_lower.contains(kw));

    // ... later ...

    let research_keywords = ["research", "analyze", "understand", "learn", "compare"];
    let is_research_goal = research_keywords.iter().any(|kw| objective_lower.contains(kw));

    if is_research_goal {
        let has_destructive = program.steps.iter().any(|s| {
            if let Step::Shell { cmd, .. } = s {
                cmd.contains("rm ") || cmd.contains("delete") || cmd.contains("drop")
            } else { false }
        });
        // ...
    }
}
```

This violates **Architectural Rule 1**:

> "Never implement routing, classification, or behavior selection through hardcoded word triggers."

The current implementation:
1. Scans the objective for 8 "action keywords" to decide if the goal is action-oriented
2. Scans for 5 "research keywords" to decide if the goal is research-oriented
3. Scans shell commands for `rm `, `delete`, `drop` to detect destructive operations
4. Produces verdicts that influence program execution without model-based validation

This is exactly the wrong approach according to Rule 1 and Rule 13 (hardening):
- A user saying "remove duplication" triggers `is_action_goal=true` because it contains "remove"
- A user saying "study how deletes work" triggers `is_research_goal=true` and may block legitimate shell commands
- The system replaces model judgment with rigid keyword lists

## Root Cause

`guardrails.rs` was created early as a "state-aware guardrails" module. The keyword-based drift detection was a quick heuristic that was never upgraded to use the intel unit framework. The `check_no_progress` and `check_meta_planning` functions are deterministic and acceptable, but `check_step_goal_mismatch` is a Rule 1 violation.

## Proposed Solution

### Phase 1 — Remove keyword-based mismatch check

1. Delete `check_step_goal_mismatch()` entirely
2. Remove its call from `check_goal_drift()`
3. Keep `check_no_progress()` and `check_meta_planning()` — these use step counts and step types, not keyword matching

### Phase 2 — Replace with model-based drift detection (optional, post-delete)

If goal-type mismatch detection is still needed after deletion:

1. Create a focused intel unit `GoalDriftUnit` that takes:
   - Original objective
   - Current program steps (as a compact list, not raw JSON)
   - Step results summary
2. Outputs compact JSON: `{"drift_detected": true/false, "reason": "...", "confidence": 0.85}`
3. Only runs when `check_no_progress` or `check_meta_planning` has already flagged a potential issue
4. Falls back to "no drift" on any failure

This follows Rule 3: the deterministic checks (`no_progress`, `meta_planning`) are fast and safe; the model-based check only runs when there's already a signal, reducing cognitive load per call.

### Phase 3 — Harden remaining deterministic checks

1. `check_no_progress()`: ensure it doesn't flag progress falsely when evidence is being gathered
2. `check_meta_planning()`: ensure the threshold (plan_count * 2 >= total_steps) is appropriate for the complexity level

## Acceptance Criteria

- [ ] `check_step_goal_mismatch()` is deleted
- [ ] No `action_keywords`, `research_keywords`, or similar keyword lists exist in `guardrails.rs`
- [ ] `check_goal_drift()` still works using only deterministic checks (`no_progress`, `meta_planning`)
- [ ] If `GoalDriftUnit` is created, it follows the compact JSON contract and has fallback behavior
- [ ] `cargo build && cargo test` passes
- [ ] Guardrails tests are updated to not test keyword-based behavior

## Verification Plan

- `grep -n "keywords" src/guardrails.rs` → no matches
- `grep -n "contains(" src/guardrails.rs` → only matches in `check_meta_planning` (step type matching, acceptable)
- Unit test: `check_goal_drift` with objective "remove duplication" and read-only steps → does NOT flag drift (no keyword check)
- Unit test: `check_goal_drift` with 5+ steps and 0 successful modifications → still flags drift (deterministic check preserved)

## Dependencies

- `src/intel_trait.rs` (if creating GoalDriftUnit)
- `src/program.rs` (Program type)
- `src/types_core.rs` (Step enum)

## Notes

Do not keep the keyword lists as "fallback heuristics." The docs say:

> "If you are checking user text for words to force a route, you are violating Elma's philosophy."

The deterministic checks (`no_progress`, `meta_planning`) are sufficient for a first line of defense. If they produce false positives, tighten their thresholds rather than adding keyword exceptions.

The shell command `contains("rm ")` check inside `check_step_goal_mismatch` is especially dangerous because it ignores the shell preflight system, which already has more sophisticated destructive command detection. This duplication is a reliability hazard.
