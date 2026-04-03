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
- Baseline `hello` conversational turns could stall in classification or heavyweight chat finalization
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
5. Simple conversational turns are paying for too many model round-trips and can stall before final output.

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
- [x] Reproduced baseline `hello` failure in live CLI mode
- [x] Added timeout-aware routing/finalization calls on the baseline chat path
- [x] Added conservative `CHAT` fallback when route confidence is low but speech act is conversational
- [x] Verified baseline `hello` succeeds in real CLI mode
- [x] Reduced startup workspace brief size for routing/planning context
- [x] Added startup workspace facts for repo root, shell name, and tool availability
- [x] Removed unused router `n_probs` payload bloat from live classifier calls
- [x] Added live program-policy rejection for overbuilt task-level plans
- [x] Verified `S000E` now reaches real shell execution in CLI mode
- [x] Standardized `selector` as a managed grammar-constrained intel unit for shell fallback selection
- [x] Fixed shell placeholder handoff for selected values in `S000E` fallback execution
- [x] Verified `S000E` now completes end-to-end in real CLI mode
- [x] Reworked route aggregation so `speech_act=CHAT` cannot override a confident workflow/action route
- [x] Reproduced and narrowed `S000F` to a post-selection verification/sufficiency seam
- [x] Added a grounded candidate-selection fallback workflow for file-selection stress prompts

## Results
- Root cause 1: `command_exists()` used `--version`, which falsely rejected valid macOS tools like `ls` and blocked the direct-shell fast path.
- Root cause 2: `orchestrate_with_retries()` discarded the already-built initial program on attempt 1, so direct shell programs were regenerated instead of executed.
- Root cause 3: goal-drift detection still used keyword-overlap heuristics, which falsely flagged successful direct shell execution as context drift and triggered malformed refinement.
- Root cause 4: remaining byte-slice truncation paths in `routing_parse.rs` and `guardrails.rs` were still vulnerable to Unicode boundary panics.
- Root cause 5: trivial `CHAT` turns were still using a multi-call routing/reflection/finalization chain, so a simple greeting could stall before Elma ever replied.
- Root cause 6: `S000E` fallback selection used an under-specified selector contract and oversized evidence payloads, so selection frequently returned empty items.
- Root cause 7: shell fallback command generation emitted a malformed placeholder handoff (`{sel1|raw}`), so the selected value never reached the call-site search step correctly.

## Verified Fixes
- `cargo test` passes with 146 tests
- Real CLI reproduction now executes and answers `ls -ltr` successfully
- Added regression tests for:
  - direct shell success not triggering drift
  - Unicode-safe truncation helpers
  - Unicode-safe parse preview path
- `printf 'hello\n/exit\n' | cargo run` now returns a real greeting through the lightweight chat path

## Extended CLI Probe Results
- `pwd` executes and returns the working directory through the direct shell path.
- `ls src` executes and returns directory contents through the direct shell path.
- `git status --short` now executes and returns real repository status after:
  - allowing direct-shell fast path when workflow planning is `DIRECT/LOW` even if complexity is conservative
  - raising inline shell capture ceiling from `128 KiB` to `1 MiB`
- `hello` now classifies conservatively as `CHAT`, skips reflection for pure `reply_only` chat, and returns a greeting instead of stalling.
- Workspace startup context is now smaller and more decision-useful:
  - concise repo/shell/tool facts in `workspace.txt`
  - bounded workspace tree in `workspace_brief.txt`
- Router calls no longer request unused logprob payloads, reducing response bodies from ~500-600 KB to ~10 KB in live traces.
- `S000E_Sequential_Logic` no longer dies in classifier/planner stall:
  - it reaches ladder/planning
  - rejects invalid `plan`-only task-level programs
  - falls back into executable shell probing
  - reaches real shell artifacts under the sandbox session
  - uses the standardized `selector` unit to choose one function candidate
  - correctly injects the selected function into the shell call-site search
  - completes with a grounded final answer in real CLI mode
- `S000F_Select_Primitive` no longer hallucinates a repo-foreign answer or collapse into pure chat:
  - it routes through workflow execution
  - uses a grounded five-step shell/select/select/reply fallback
  - currently stalls at the last-mile verification/sufficiency layer, which still underrates the completed selection workflow

## Rollback Check
- Any failed experimental changes must be reverted before leaving troubleshooting phase.

## Exit Criteria
- `cargo run` starts cleanly
- A trivial shell command like `ls -ltr` executes without orchestration stall
- No new regressions in tests or scenario probes
