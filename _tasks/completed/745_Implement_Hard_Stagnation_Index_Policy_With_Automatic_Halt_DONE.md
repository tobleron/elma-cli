# Task 745: Implement Hard Stagnation Index Policy With Automatic Halt

## Type

Architecture / Reliability / Hardening

## Severity

Critical

## Scope

Stop policy, tool loop, approach engine, user interaction

## Problem

**Architectural Rule 13** mandates:

> "Implement a 'Stagnation Index' as a hard policy. If a task hasn't achieved progress via verifiable outcomes after X iterations, the agent MUST halt and request human intervention."

This rule is **not implemented** in the codebase. The current `stop_policy.rs` has:

- `stagnation_runs` counter
- `max_stagnation_cycles: 8` in `StageBudget`
- `StopReason::ModelProgressStalled`
- `StopReason::RepeatedNoNewEvidence`
- `StopReason::RepeatedSameCommand`

But there is **no Stagnation Index calculation**, and critically, **no reliable way to distinguish a long-running agent that is making slow progress from an agent that is looping without new evidence**.

When stagnation is detected, the tool loop simply sets `StopReason::RepeatedNoNewEvidence` or similar and exits. The final answer pipeline then tries to synthesize an answer from whatever evidence exists. The user receives a potentially wrong or incomplete answer with no indication that the agent was stuck.

This task must preserve Elma's default long-running autonomous behavior. The goal is not to stop early; the goal is to detect evidence-free loops and switch approach or halt only when continued autonomous work is no longer justified.

This violates:
- **Rule 13**: hard stagnation policy
- **Rule 2**: reliability over speed — silently producing a weak answer is not reliable
- **Rule 11**: eliminate retry loops and stale recovery behavior before improving human-style robustness
- **AGENTS.md Rule 6**: transcript-native operational visibility — stagnation triggers are hidden in trace logs, not visible transcript rows

## Root Cause

The stop policy was designed as a "budget enforcer" (max iterations, max wall clock) rather than a "progress verifier." Stagnation detection was added incrementally as stop reasons but never wired to a hard halt with user notification.

## Proposed Solution

### Phase 1 — Define Stagnation Index

In `src/stop_policy.rs`, add a `StagnationIndex` struct:

```rust
pub(crate) struct StagnationIndex {
    /// Iterations since last verifiable progress
    iterations_without_progress: usize,
    /// Last recorded progress hash (hash of successful evidence state)
    last_progress_hash: Option<u64>,
    /// Maximum allowed iterations without progress before hard halt
    max_stagnant_iterations: usize,
    /// Whether a hard halt has been triggered
    halted: bool,
}
```

"Verifiable progress" means:
- A new file was successfully read (and produced non-empty content)
- A shell command succeeded with non-empty output
- A search returned new results
- A file was successfully written or edited
- The evidence ledger gained a new entry with `quality != Weak`

"No progress" means:
- Tool calls fail repeatedly
- Tool calls succeed but produce empty/duplicate output
- The model calls the same tool with the same arguments
- The model calls `respond` without gathering new evidence

### Phase 2 — Compute progress hash

After each successful tool call, compute a hash of:
- Tool name
- Arguments (normalized)
- Exit code / success status
- Output hash (first 1KB)

If this hash is new, progress occurred. If it matches a previous hash, it's duplicate output.

### Phase 3 — Approach shift before hard halt

When `iterations_without_progress >= max_stagnant_iterations`:

1. First attempt a bounded sibling approach if the active work graph still has viable alternatives.
2. Reset the stagnation index only if the sibling approach produces verifiable new evidence.
3. If all bounded alternatives are exhausted, set `halted = true`.
4. **Do not** proceed to final answer synthesis.
5. Instead, produce a specific user-facing message:
   ```
   [Stagnation Detected]

   I was unable to make progress on this task after {N} iterations.

   Evidence gathered so far:
   - {count} tool calls made
   - {count} successful reads
   - Last successful action: {description}

   Suggested next steps:
   - Rephrase your request with more specific file paths
   - Break the task into smaller steps
   - Verify the workspace contains the expected files
   ```
6. Log the stagnation event to the session transcript as a **visible, non-collapsible** row.
7. Set the turn outcome to a special `TurnOutcome::StagnationHalt` variant.

### Phase 4 — Configurable threshold

Add to `config/runtime.toml`:

```toml
[stagnation]
max_stagnant_iterations = 10
small_model_max_stagnant_iterations = 14
max_sibling_approaches_before_halt = 2
```

### Phase 5 — Integration with approach engine

If the approach engine is active, a stagnation halt should:
1. Mark the current approach as `Failed`
2. Fork a sibling approach only while `max_sibling_approaches_before_halt` has not been exhausted
3. Report the halt to the user only after bounded alternatives fail to make progress

This follows Rule 4b: "When an approach fails, the system forks a new sibling approach... Each approach is a separate branch." But if all bounded branches are stagnating, human intervention is required.

## Acceptance Criteria

- [ ] `StagnationIndex` struct exists with verifiable progress tracking
- [ ] Progress hash is computed after each successful tool call
- [ ] After `max_stagnant_iterations` without progress, the agent shifts to a bounded sibling approach when available
- [ ] The agent halts only after bounded alternatives are exhausted or the failure is non-recoverable
- [ ] Halt produces a user-visible message with evidence summary and suggestions
- [ ] Halt does NOT synthesize a final answer from weak/incomplete evidence
- [ ] Stagnation event is visible in the transcript (not hidden in trace logs)
- [ ] Threshold is configurable via `config/runtime.toml`
- [ ] Approach engine respects halt and does not fork beyond the configured sibling-approach cap
- [ ] `cargo build && cargo test` passes
- [ ] Unit tests verify halt triggers correctly and doesn't trigger prematurely

## Verification Plan

- Unit test: 5 iterations with unique successful evidence → no halt
- Unit test: 6 iterations with repeated same-command failures → halt triggered
- Unit test: 3 iterations with `respond` abuse → halt triggered
- Integration test: mock tool loop that always fails → agent halts with visible message
- Scenario test: user asks for a non-existent file → agent attempts `read`, `exists`, `search`, then halts instead of hallucinating an answer

## Dependencies

- `src/stop_policy.rs` (primary implementation site)
- `src/tool_loop.rs` (integration point)
- `src/approach_engine.rs` (approach failure handling)
- `src/evidence_ledger.rs` (progress verification)
- `src/ui_terminal.rs` (user-visible halt message)

## Notes

This is a **behavior change**, not just an internal refactor. Users will see the agent stop and ask for help instead of producing a weak answer. This is the correct behavior per Rule 13 and Rule 2.

Do not implement this as a "soft warning" that still allows finalization. The docs say "MUST halt and request human intervention" — this is a hard stop.

The stagnation message should be concise (3-5 lines) and actionable. It should not dump raw tool output.
