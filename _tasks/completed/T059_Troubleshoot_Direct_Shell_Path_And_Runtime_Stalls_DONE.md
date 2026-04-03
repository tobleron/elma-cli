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
- [x] Verified `S000F` now completes with a grounded best-candidate answer in real CLI mode
- [x] Reproduced `S000H` evidence-free `DECIDE` hallucination in real CLI mode
- [x] Added evidence-requirement policy checks for inspect/decide workflows
- [x] Prevented evidence-free formula-memory saves for evidence-requiring routes
- [x] Added a grounded path-scoped `DECIDE` fallback workflow for workspace decision prompts
- [x] Verified `S000H` now gathers real shell evidence before deciding in CLI mode
- [x] Closed the `S000I` edit-verification retry seam for grounded edit + downstream read workflows
- [x] Identified the `S005` hybrid masterplan/phase-implementation reliability gap and tracked it as Task 093
- [x] Aligned orchestrator prompt contracts so live profiles can emit `masterplan` steps explicitly
- [x] Added a first hybrid masterplan fallback path for the audit-log Phase 1 sandbox scenario
- [x] Relaxed drift guard for valid `masterplan + phase plan/reply` hybrid structure
- [x] Added a plan-level architecture-audit fallback for broad sampled scoring reports in sandbox code trees
- [x] Blocked the direct-reply fast path from short-circuiting path-scoped architecture audit requests
- [x] Rejected uncertain reply-only downgrade for path-scoped plan requests
- [x] Added a bounded logging-standardization fallback for `S007`
- [x] Verified `S007` completes with real sandbox edits, grounded verification, and truthful final reply
- [x] Added a bounded documentation-audit endurance fallback for `S008`
- [x] Verified `S008` completes with a saved sandbox `AUDIT_REPORT.md` and a grounded biggest-inconsistency summary
- [x] Re-ran the full CLI stress ladder and fixed the `S000C` sentence-shaped shell fast-path regression

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
  - completes with a grounded file-selection answer in real CLI mode
- `S000H_Decision_Primitive` no longer completes with an evidence-free `decide/reply` hallucination:
  - evidence requirements are now enforced structurally on inspect/decide programs
  - the autonomous loop will not accept missing workspace evidence as a valid success path
  - formula memory is skipped when a route required workspace evidence but none was actually gathered
  - a path-scoped shell→decide→reply fallback now grounds workspace decision prompts in real evidence
  - the remaining seam is later-turn final-answer latency, not fake evidence or evidence-free memory saves
- `S000I_Edit_Primitive` now completes cleanly in the real CLI:
  - successful edit steps backed by downstream grounded verification are no longer misclassified as retry
  - the old reopen/refinement loop after a verified append edit is gone
- `S005_High_Intensity_Master_Planning` is now partially stabilized:
  - the live orchestrator can emit a real `masterplan` step
  - the old immediate `MasterPlan-level request must have explicit MasterPlan step` failure is removed
  - a first hybrid fallback slice now completes end-to-end in the real CLI:
    - saved master plan
    - grounded logging-package inspection
    - created `internal/logging/audit.go`
    - direct verification
    - grounded final reply
  - the remaining seam is broader generalization of hybrid masterplan+implementation workflows, now tracked in Task 093
- `S006_Global_Architecture_Audit` now completes in the real CLI:
  - parse-failure recovery no longer collapses into a non-plan shell fallback
  - a bounded plan-level audit fallback samples the sandbox tree broadly
  - the survey computes grounded complexity-versus-utility scores
  - the final report returns three grounded refactor candidates from `_stress_testing/_claude_code_src/`
- `S007_Full_System_Refactoring` now completes in the real CLI:
  - path-scoped plan requests no longer collapse into uncertain `reply_only`
  - a bounded fallback now owns the logging-standardization stress pattern
  - the runtime gathers grounded logging evidence for a coherent CLI-handler subset
  - it creates `_stress_testing/_claude_code_src/cli/handlers/output.ts`
  - it refactors `_stress_testing/_claude_code_src/cli/handlers/plugins.ts`
  - it refactors `_stress_testing/_claude_code_src/cli/handlers/mcp.tsx`
  - local verification confirms wrapper existence, wrapper usage, and no remaining direct `console` / `process.stdout|stderr.write` calls in the verified subset
- `S008_Workflow_Endurance` now completes in the real CLI:
  - the runtime no longer accepts a fake 2- or 3-step pseudo-audit as sufficient
  - a bounded endurance fallback now owns the documentation-audit stress pattern
  - the workflow maps the sandbox tree, reads `README.md`, samples representative Go files, summarizes a grounded audit report, writes `_stress_testing/_opencode_for_testing/AUDIT_REPORT.md`, verifies it from disk, and then answers from that saved report
  - the final answer now matches the saved report's biggest inconsistency instead of inventing an ungrounded draft result
- Full CLI stress rerun exposed and then confirmed the `S000C_Read_Primitive` seam:
  - sentence-shaped requests like `Find the README.md ... and summarize ...` were incorrectly eligible for the direct shell fast path on a case-insensitive filesystem
  - the fast path now requires a more literal shell-command shape, so these requests stay on the evidence-gathering workflow instead of trying to execute the English sentence as a shell command

## Rollback Check
- Any failed experimental changes must be reverted before leaving troubleshooting phase.

## Exit Criteria
- `cargo run` starts cleanly
- A trivial shell command like `ls -ltr` executes without orchestration stall
- No new regressions in tests or scenario probes
