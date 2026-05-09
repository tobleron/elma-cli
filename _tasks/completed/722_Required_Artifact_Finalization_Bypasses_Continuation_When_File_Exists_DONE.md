# Task 722: Required Artifact Finalization Bypasses Continuation When File Exists

## Type

Autonomy / Stop Policy / Artifact Verification

## Severity

High

## Evidence

Round 7 traces show:

```text
tool_loop: stopping reason=iteration_limit_reached
tool_loop: budget continuation deferred to required artifact finalization (all artifacts exist or continuations exhausted)
finalization_stage=deterministic_artifact_completion
```

Example:

- `project_tmp/round7_sessions/prompt_01_s_1778142976_493585000/trace_debug.log`
- `project_tmp/round7_sessions/prompt_05_s_1778143178_46509000/trace_debug.log`

The required artifact file exists, but its content is only an evidence-recovery artifact.

## Problem

Task 718 appears to treat artifact existence as enough to bypass continuation. This is wrong. Continuation must depend on artifact quality/state, not only path existence.

## Requirements

- Replace `all artifacts exist` checks with `all artifacts verified complete`.
- Consider these incomplete:
  - recovered evidence dumps
  - empty files
  - files missing requested sections
  - files generated from stale previous sessions
- If the artifact exists but is partial, run bounded continuation or structured deterministic synthesis.
- Record why continuation was attempted or skipped.
- Update session status to `partial` or `failed` if the objective remains incomplete after continuation budget.

## Acceptance Criteria

- [ ] Existing partial artifacts do not bypass continuation.
- [ ] Trace distinguishes `artifact_exists` from `artifact_verified_complete`.
- [ ] Prompt 01/05-style sessions either produce complete reports or end partial, not completed.
- [ ] Tests cover existing complete artifact, existing partial artifact, and missing artifact.

