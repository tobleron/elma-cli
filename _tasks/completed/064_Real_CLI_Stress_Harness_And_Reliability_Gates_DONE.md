# Task 064: Real CLI Stress Harness And Reliability Gates

## Priority
**P0 - VERIFICATION INFRASTRUCTURE**
**Master Plan:** Tracked under Task 095, Phase 1 (Task 2: Close active stress harness gaps)

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
- Session `s_1775236515_901751000` showed a third missing gate that is now important:
  - `S000B` produced the right entry-point filename (`main.go`) but failed to preserve the exact grounded relative path from evidence
  - the CLI stress harness should treat path-softening as a real grounding failure for file-identification prompts

## Progress Notes
- The CLI runner now captures session ids, extracts final answers, prints failure context, and fails on timeout / no-progress termination instead of labeling those runs as passed.
- Semantic validation has started for benchmark-specific prompts:
  - entry-point prompts now fail if the final answer does not preserve a grounded file path
  - 3-bullet summary prompts now fail if the final answer is not actually 3 bullets
  - candidate-selection prompts now fail if the final answer does not contain the requested number of grounded file candidates
- The first honest failure after these gates is `S000B`, which confirms the harness is now surfacing a real runtime-quality seam instead of masking it.
- The current remaining live seam is narrower than before:
  - the `S000B` workflow now reaches a grounded 4-step evidence path in the real CLI
  - targeted tests for exact-path preservation and exact-selection retry suppression now pass
  - selector normalization now prefers the shallow grounded path when basename matches are ambiguous
  - live CLI `S000B` now returns `_stress_testing/_opencode_for_testing/main.go`
  - the next stress-harness step is to rerun the CLI ladder and capture the first honest failure after `S000B`

## Additional Ad Hoc CLI Findings
- Real human-style probing outside the formal stress prompts still exposes reliability gaps:
  - sloppy greeting `yo elma u there??` returned `No steps observed for this request.` instead of a normal conversational greeting
  - casual self-description prompt `sup buddy just tell me what u do real quick` returned a generic `AI language model providing quick assistance and information.` answer, which is stable but too generic for Elma's intended identity
  - casual shell-style request `umm can u pls list src and dont overdo it` under-executed into a direct answer with incorrect file names like `src/config.rs` and `src/util.rs`, which indicates a real human-phrasing route/grounding seam outside the formal stress ladder
  - a bounded multi-instruction sandbox request incorrectly routed to `CHAT` and hallucinated a repo purpose plus `scripts/run_stress_test.sh` as the entry point, proving that multi-instruction natural-language requests are still not reliably classified into evidence-grounded workflows
- These findings confirm Elma is structurally much stronger than before, but not yet robust enough against sloppy human phrasing to claim broad conversational reliability.

## Latest Progress (2026-04-04 Phase 1 Gate)
- `run_stress_cli.sh` built as real CLI stress runner with semantic validation gates
- Fixed macOS compatibility (no `timeout` command — added detection logic)
- Real CLI verification results:
  - S000A (chat baseline): over-orchestrates (runs `rg --files` for simple chat), final answer grounded
  - S000B (shell primitive): CHAT route instead of SHELL, lists files but entry point identification implicit
  - S002 (recursive discovery): `rg --type source` unsupported, retry loop replays same command instead of changing strategy
  - Combined read+summary+entry-point: hallucinated `scripts/run_tests.sh` — CHAT route answered without reading file
  - Sloppy greeting: ✅ `Hi there!` — perfect
  - Casual scoped listing: ✅ bounded `ls src` workflow, real evidence, truncated output
- `cargo build` clean, `cargo test` 220 passed, intention scenarios 20/20, reliability probe 30/30

### Previous Progress (from Task 058)
- Runtime profile/config writes are now atomic, which reduces transient parse failures during concurrent profile sync/load activity.
- A new combined bounded workflow exists for:
  - read README
  - produce exactly 2 bullets
  - identify the primary entry point by exact path
  - no modification

## Current Honest Remaining Seams
- The combined sloppy multi-instruction sandbox prompt is still not fully reliable end to end:
  - it now executes the grounded workflow instead of collapsing immediately to pure chat
  - but the README-summary phase can still drift semantically, and the final answer is not yet consistently preserving both the requested 2-bullet purpose summary and the exact entry-point path together
- Casual self-description prompts still drift into a generic identity response instead of a stable Elma-specific self-description.
