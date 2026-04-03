# Task 063: Real CLI Stress Harness And Reliability Gates

## Priority
**P0 - VERIFICATION INFRASTRUCTURE**

## Objective
Create a true CLI-grounded stress harness so Elma is validated through `cargo run` sessions, not only through orchestrator/model path approximations.

## Why This Exists
The project already has stress prompts and scenario runners, but recent debugging proved that model-path checks can look healthy while the real CLI still fails. We need the canonical reliability gate to be the actual CLI runtime.

## Problems To Solve
- Current stress validation is not yet consistently end-to-end through the CLI.
- Session outputs, traces, shell artifacts, and final answers are not yet evaluated as one integrated pass/fail unit.
- There is no strong automated gate for “grounded answer + sandbox confinement + honest failure behavior” in real CLI sessions.
- The current CLI runner can mislabel failures as passes:
  - timeout / no-progress termination still prints `PASSED`
  - it does not yet fail on sessions that never produced a trustworthy final answer
  - it does not yet compare raw artifacts against downstream summarized claims

## Scope
- Build a real CLI stress runner for `_stress_testing/`.
- Run prompts through `cargo run`.
- Capture:
  - session id
  - final answer
  - trace file
  - shell/decision/read/search artifacts
- Add pass/fail rules for:
  - final answer presence
  - evidence grounding when required
  - sandbox confinement
  - no repo-self-modification
  - no unsupported claims
  - no timeout/no-progress false positives
  - no “passed” result when the final answer came from a broken or partially executed workflow
  - no contradiction between raw shell/read artifacts and downstream compacted/reported claims
- Support incremental gating from primitive to harder prompts.

## Deliverables
- A CLI-grounded stress runner.
- A pass/fail report format suitable for repeated local use.
- A curated incremental gate order for stress prompts.
- Documentation for how to interpret failures and inspect sessions.
- A failure classification that distinguishes:
  - routing over-execution
  - stale retry loops
  - artifact-grounding contradictions
  - timeout / hang
  - sandbox violation

## Acceptance Criteria
- Stress prompts can be run through the real CLI in sequence.
- Failures surface concrete session evidence, not just “scenario failed.”
- The runner is safe for local sandboxed stress repos.
- It becomes the authoritative stress-validation path moving forward.
- The runner cannot report `PASSED` after timeout/no-progress termination.
- The runner fails when trace/artifacts show grounded-evidence contradictions even if a final answer string exists.

## Additional Session Evidence
- Session `s_1775234563_585917000` showed the current harness weakness indirectly:
  - `S000C` hit a broken workflow path and timeout/no-progress behavior
  - the shell runner still labeled the test as passed
- Session `s_1775235404_589084000` showed a second missing gate:
  - downstream compacted evidence claimed a successful rename even though raw shell artifacts only showed an unsupported `rg` flag failure
  - the stress harness should eventually flag this contradiction automatically
