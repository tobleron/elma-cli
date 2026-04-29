# Task 381: DSL Retry Repair And Stop Policy Integration

**Status:** pending
**Priority:** critical
**Suite:** Compact DSL Model-Output Migration
**Depends on:** Tasks 377, 378, 380
**Blocks:** Task 384

## Objective

Integrate DSL parse, semantic validation, safety validation, executor failures, and edit failures into Elma's retry and stop-policy machinery. Invalid DSL should produce compact corrective observations, not loops, hallucinated tool results, or hidden trace-only failures.

## Required Deliverables

- Unified DSL repair observation flow.
- Retry budget integration for DSL parse/validation failures.
- Stop-policy signals for repeated invalid DSL, repeated unsafe actions, repeated failed edits, and repeated no-progress observations.
- Transcript-visible operational rows for DSL repair and stop reasons.

## Failure Classes

At minimum classify:

- `invalid_dsl`
- `unsafe_path`
- `unsafe_command`
- `invalid_edit`
- `executor_failed`
- `empty_observation`
- `repeated_action`
- `stagnation`
- `max_iterations`
- `finalization_failed`

These classes should be structural enum values where practical, not inferred from arbitrary strings.

## Repair Observation Contract

Observations must be short and directive:

```text
INVALID_DSL
error: unknown command "READ"
expected: R, L, S, Y, E, X, ASK, DONE
return exactly one valid command
```

```text
INVALID_EDIT
error: OLD block matched 3 times in src/main.rs
next: use a larger unique OLD block
```

```text
UNSAFE_COMMAND
error: command is not allowed by policy
allowed: cargo check, cargo test, cargo fmt, cargo clippy, git diff, git status, ls, rg, grep
return a safe command or use another DSL action
```

## Implementation Steps

1. Map `DslErrorCode` and executor failure types into stop-policy/failure-class types.
2. Add per-turn and per-session counters for repeated invalid DSL and repeated failed actions.
3. Feed repair observations back as user/tool-equivalent context without pretending execution succeeded.
4. Make repair events transcript-visible and session-persisted.
5. Force clean-context finalization when invalid/repeated DSL exceeds budget.
6. Remove JSON repair unit dependency from migrated paths.
7. Add tests for repeated bad output and recovery.

## Verification

Required commands:

```bash
cargo fmt --check
cargo test stop_policy
cargo test tool_loop
cargo test agent_protocol
cargo test evidence_ledger
cargo test session_flush
cargo check --all-targets
```

Required coverage:

- invalid DSL gets one compact repair observation
- repeated invalid DSL triggers bounded stop/finalization
- unsafe path/command does not execute
- invalid edit suggests next command
- repair observations are visible in transcript/session
- successful retry after repair resets relevant failure counters

## Done Criteria

- Malformed model output is a controlled, visible, recoverable state.
- Retry logic is DSL-native and no longer depends on JSON repair.
- Stop policy prevents invalid-output loops without blaming the model.

## Anti-Patterns

- Do not call a model to repair DSL before deterministic parser feedback has been tried.
- Do not hide parse/validation failures only in trace logs.
- Do not classify safety failures by broad free-form substring matching.
