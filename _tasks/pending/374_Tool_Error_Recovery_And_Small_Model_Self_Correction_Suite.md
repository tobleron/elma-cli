# Task 374: DSL Error Recovery And Small Model Self-Correction Suite

**Status:** pending
**Priority:** high
**Suite:** DSL Protocol And Skills Certification
**Depends on:** Task 365 (DSL protocol self-test harness), Task 371 (DSL loop coherence), Task 381 (DSL retry/repair integration)

## Objective

Certify that Elma recovers from invalid DSL, unsafe actions, and executor failures by providing compact repair observations, decomposing, narrowing, rereading, or choosing a safer action instead of looping, hallucinating, or blaming the model.

## Required Deliverables

- prompt scenarios under `tests/dsl/prompts/error_recovery.md`
- regression tests for repeated failures and strategy shifts
- failure-class report integrated with the self-test harness

## Built-In Elma CLI Prompt Pack

```text
Search for a sentinel using an intentionally too-broad strategy first, then narrow the search if the result is too large. Find TOOL_RECOVERY_ALPHA.
```

```text
Try to read a file using an incorrect path similar to real_fixture.txt. When it fails, search for the correct path and then read it.
```

```text
Attempt a replace operation with text that does not exist. Recover by rereading the file and identifying the correct text before deciding whether to edit.
```

```text
Run a shell command that fails because the path is wrong, then switch to search/read rather than repeating the same failing command.
```

```text
If an action/adapter is disabled or unavailable, explain the limitation and use the best available local alternative. Do not invent a successful action result.
```

## Verification

Required commands:

```bash
cargo fmt --check
cargo test stop_policy
cargo test orchestration_retry
cargo test agent_protocol
cargo test tool_loop
cargo test evidence_ledger
cargo build
```

Prompt pass criteria:

- repeated identical failures are detected
- model receives actionable compact repair guidance
- later action use changes strategy meaningfully
- final answer identifies unresolved blockers honestly
- no final answer claims evidence that does not exist

## Done Criteria

- Common DSL/action failures lead to bounded recovery.
- Every observed recovery failure has an automated regression.
- Small-model weakness is handled through system decomposition, not prompt bloat.
