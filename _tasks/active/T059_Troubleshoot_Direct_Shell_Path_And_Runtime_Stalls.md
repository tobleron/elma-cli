# Task T059: Troubleshoot Direct Shell Path And Runtime Stalls

## Priority
**P0 - TROUBLESHOOTING**
**Created:** 2026-04-03
**Triggered by:** Real CLI session failures and stalls during `cargo run`

## Status
**ACTIVE**

## Problem
Real Elma CLI sessions are still unstable on simple shell requests.

Observed failures:
- `cargo run` startup was broken by mixed-schema config loading (`model_behavior.toml`)
- Recent live session stalled on a trivial `ls -ltr` request before any shell step executed
- Older sessions show Unicode char-boundary panics in runtime parsing/truncation paths

## Evidence
- `sessions/s_1775176560_799725000/trace_debug.log`
- `sessions/s_1775161534_182953000/error.json`
- `sessions/s_1775157998_142462000/error.json`

## Hypotheses
1. Direct shell requests are over-routed into planning/recovery instead of taking a minimal execution path.
2. Formula alignment is overriding valid direct execution into `inspect_reply` for simple shell commands.
3. Orchestrator JSON generation is still too fragile for trivial shell programs.
4. Runtime string slicing paths still contain Unicode-unsafe truncation/parsing logic.

## Phase 0 Plan
- Reproduce the live CLI failure path from session evidence.
- Identify whether the stall occurs before orchestration, during repair/recovery, or before first shell execution.
- Add the smallest safe direct-shell fast path for trivial shell commands.
- Verify the fix in real CLI mode, not only prompt runner mode.
- Audit and fix the Unicode char-boundary panic sites if still reachable.

## Experiment Log
- [x] Reproduced real CLI startup failure from `cargo run`
- [x] Identified mixed-schema config loading bug around `model_behavior.toml`
- [x] Fixed startup so `cargo run` reaches interactive prompt again
- [x] Reproduced simple shell-request stall in live CLI path
- [x] Implemented direct-shell fast path for trivial command execution
- [x] Verified no regression with `cargo test`
- [x] Verified direct-shell behavior in live CLI session (`printf 'ls -ltr\n/exit\n' | cargo run`)
- [x] Checked Unicode char-boundary panic sites and patched unsafe preview/truncation paths

## Results
- Root cause 1: `command_exists()` used `--version`, which falsely rejected valid macOS tools like `ls` and blocked the direct-shell fast path.
- Root cause 2: `orchestrate_with_retries()` discarded the already-built initial program on attempt 1, so direct shell programs were regenerated instead of executed.
- Root cause 3: goal-drift detection still used keyword-overlap heuristics, which falsely flagged successful direct shell execution as context drift and triggered malformed refinement.
- Root cause 4: remaining byte-slice truncation paths in `routing_parse.rs` and `guardrails.rs` were still vulnerable to Unicode boundary panics.

## Verified Fixes
- `cargo test` passes with 146 tests
- Real CLI reproduction now executes and answers `ls -ltr` successfully
- Added regression tests for:
  - direct shell success not triggering drift
  - Unicode-safe truncation helpers
  - Unicode-safe parse preview path

## Rollback Check
- Any failed experimental changes must be reverted before leaving troubleshooting phase.

## Exit Criteria
- `cargo run` starts cleanly
- A trivial shell command like `ls -ltr` executes without orchestration stall
- No new regressions in tests or scenario probes
