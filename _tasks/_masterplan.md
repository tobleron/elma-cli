# Master Plan

**Last updated:** 2026-04-30

This is the execution index for all current pending tasks. Use it to choose work in dependency order, not as a replacement for each task file. Each task file remains the source of implementation detail, verification commands, and done criteria.

## Operating Rules

- Move a task from `_tasks/pending/` to `_tasks/active/` before implementation.
- Complete the task's phases here as progress markers, but use the task file for exact scope.
- Do not mark a task complete until its own verification section passes.
- Do not modify `src/prompt_core.rs` or `TOOL_CALLING_SYSTEM_PROMPT` unless a task explicitly has user approval for that change.
- Prefer completing each wave in order. Within a wave, tasks can run in parallel only when their dependencies do not overlap.

## Current Focus (Post-DSL Stabilization)

The current objective is to ensure Elma's *basic* workflow is stable under the new compact DSL protocol before taking on feature expansion.

**Current pending tasks (keep in `_tasks/pending/`):**
- Task 385: Live DSL runtime smoke harness and transcript capture
- Task 386: DSL action command timeouts and output budgets
- Task 387: Intel DSL profile repair and live smoke gate
- T388: Restore thinking streaming and terminal command display in DSL tool loop (regression) [DONE]
- T389: Restore evidence-grounded final summary in DSL DONE action path (regression) [DONE]
- T390: Fix small-model DSL repair confusion and ASK misuse in tool loop [DONE]

**Recently fixed core regressions (not a numbered task, but required for stability):**
- Align the live runtime with the approved compact action DSL: the tool loop no longer advertises provider-native function tools during DSL turns.
- Parse model action text with `parse_action_dsl`, reject malformed/empty/prose output, and feed compact repair observations back through bounded stop policy.
- Restore ratatui command/action visibility for DSL `X`, `L`, `R`, `S`, `Y`, and `E` actions via `ToolStarted`/`ToolFinished` transcript rows.
- Preserve the caller's `reasoning_format=none` setting in the streaming DSL action loop so executable actions stay in response content instead of being pushed into hidden reasoning/prose.
- Stop rendering raw compact-action planning/thinking deltas in the TUI; users now see the operational rows and final answer instead of failed internal DSL attempts.
- Prevent debug-trace reasoning previews from writing yellow stderr lines while the ratatui UI is active.
- Bypass the action DSL loop for model-classified `CHAT` turns so greetings like `hi` receive natural plain-text responses instead of workflow-status summaries.
- Harden DSL action execution: read/list/search paths are workspace-scoped, search no longer uses shell interpolation, `X` uses strict command policy, and `E` requires read-before-edit plus snapshot creation.
- Normalize directory reads (`R path="dir"`) into shallow list actions before execution and before transcript emission, preventing directory-read failures when the model picked the wrong inspection action.
- Default `L path="..."` to depth `1` instead of depth `2`, keeping basic directory inspection shallow and predictable.
- Bound DSL list output to a compact model-facing budget with explicit truncation text instead of flooding the model context.
- Add one bounded DSL repair retry for migrated intel units before deterministic fallback, with repair wording that separates the error code from the expected DSL contract.
- Add one bounded DSL repair retry for router classifiers before conservative fallback, so basic routing can recover as DSL instead of immediately misclassifying on a truncated/prose first attempt.
- Adapt router classifier request options to the model behavior profile: models with `none_final_clean=false` and separated auto reasoning now use `reasoning_format=auto` plus a sufficient token floor so the final DSL line can reach `content`.
- Apply the same model-behavior-aware reasoning format to generic intel DSL execution instead of forcing `none` for every structured unit.
- Remove remaining model-output JSON contract in decomposition by switching `Masterplan` generation to compact DSL and making decomposition best-effort.
- Restore DSL-era routing correctness for short operational inputs by re-enabling router-based classification (DSL router outputs) and setting `evidence_required` for all non-CHAT routes.
- Restore tool/shell operational visibility in the ratatui transcript by emitting `ToolStarted`/`ToolFinished` rows for compatibility and DSL actions inside the tool loop.
- Fix startup connectivity probe posting to `/v1/chat/completions/v1/chat/completions` (double-appended path).
- Fix persistent shell marker injection so heredocs are safe (no trailing `;` on terminator lines).
- Stabilize a flaky edit-gate unit test by serializing access to the global gate state.

**Live smoke observations (2026-04-30):**
- `sessions/s_1777580777_246585000` verified `hi` routes through DSL speech-act/router classification as `CHAT` and returns a natural response (`hi there`) without workflow-success phrasing.
- `sessions/s_1777580777_246585000/artifacts/terminal_transcript.txt` contains `TOOL TRACE RUNNING: list path=. depth=1`, confirming ratatui/transcript command visibility for `L`.
- The same trace shows `speech_act`, `workflow`, `evidence_need_assessor`, and `turn_summary` producing valid compact DSL without fallback on the basic greeting/list smoke path.
- Remaining stability work is not to expand features yet: Task 386 owns `X/S/Y` timeout and output-budget hardening, and Task 387 owns turning the basic prompt pack into a clean no-invalid-DSL live gate.

**Deferred until after the basic DSL loop is proven stable:**
- Task 337: file context tracker and stale-read gate.
- Task 339 pending duplicate: superseded by the completed Task 339 artifact.
- Task 361: workspace policy files for ignored/protected paths.

## Wave 0: Compact DSL Model-Output Migration

### 376 DSL Migration Checkpoint Inventory And Boundaries
- [x] Confirm the checkpoint commit and inventory all model-produced JSON contracts.
- [x] Classify JSON boundaries as model output, provider wire, local state, config, third-party, or fixture.
- [x] Update active task references that would preserve JSON model output.

### 377 Compact DSL Family Core Parser And Error Model
- [x] Build strict shared DSL parser primitives, sanitization, and compact repair rendering.
- [x] Reject empty, fenced, JSON/XML/YAML/TOML/prose, duplicate-field, malformed-quote, and trailing-garbage output.
- [x] Verify stable DSL parse errors and repair observations.

### 378 Action DSL Protocol And Executor Cutover
- [x] Introduce `AgentAction` and parse `R/L/S/Y/E/X/ASK/DONE` into typed Rust actions.
- [x] Replace live provider-native model tool calls with one-command DSL loop handling.
- [x] Verify parsed, validated actions execute and raw model text never executes.

### 379 DSL Path Command And Edit Safety
- [x] Enforce safe workspace-relative paths, symlink escape protection, command policy, and exact edit validation.
- [x] Restrict `X` to approved verification/development commands by default.
- [x] Verify unsafe paths, unsafe commands, and invalid edits produce compact repair observations.

### 380 Intel Units Compact DSL Migration
- [x] Replace model-produced intel-unit JSON with compact typed DSL verdict/list/scope contracts.
- [x] Remove `chat_json_with_repair` from migrated model-output paths.
- [x] Verify small-model retries use DSL repair feedback instead of JSON repair.

### 381 DSL Retry Repair And Stop Policy Integration
- [x] Route invalid DSL, unsafe actions, and invalid edits through bounded retry/repair.
- [x] Prevent unsupported `DONE`, repeated invalid output loops, and hallucinated success.
- [x] Verify compact repair observations drive strategy shifts.
### 382 DSL GBNF Grammar Family And Profile Migration
- [x] Add optional GBNF grammar assets for action and intel DSL contracts.
- [x] Update profiles and prompt contracts to request DSL only where user approval exists.
- [x] Verify constrained decoding helps but parser/validator remain authoritative.

### 383 TOML Config And Local Data Format Boundary
- [x] Make user-facing config TOML-first and document acceptable JSON boundaries.
- [x] Reframe config migration away from JSON Schema as the primary contract.
- [x] Verify no model-output JSON prompt contract is reintroduced.

### 384 DSL Certification Harness And Dead JSON Output Removal
- [x] Certify valid and malformed DSL action/intel contracts end to end.
- [x] Remove dead model-output JSON parsers, prompts, repair logic, and fallback paths.
- [x] Verify remaining JSON uses are intentional provider/local-state/config boundaries.

## Wave 1: Backlog Truth And Low-Level Hygiene

### 356 Task Backlog Drift Audit And Reconciliation
- [x] Audit pending tasks against current source and classify stale, partial, duplicate, and valid tasks.
- [x] Update or move tasks according to `_tasks/TASKS.md` without changing source behavior.
- [x] Publish a short backlog health report and verify every pending task has a current state.

### 308 Regex Crate Integration
- [x] Inventory current ad hoc pattern parsing and choose focused replacement targets.
- [x] Wire `regex` usage only where it reduces brittle parsing or improves correctness.
- [x] Add focused regression tests and run the task's cargo checks.

### 309 Strip ANSI Escapes Crate
- [x] Identify shell/tool-output paths that should sanitize ANSI before model/session use.
- [x] Replace custom stripping with `strip-ansi-escapes` through a shared helper.
- [x] Test ANSI-stripped output in shell, transcript, and evidence paths.

### 307 Tokenized Theme Enforcement
- [x] Audit UI color usage and identify hardcoded colors outside the theme module.
- [x] Route renderers through theme tokens without changing visual intent.
- [x] Run UI/theme tests and update snapshots if the change is intentional.

## Wave 2: Core Behavioral Doctrine

### 303 Offline-First Architecture
- [x] Audit all network-capable paths and startup assumptions.
- [x] Add explicit offline-first guards, degraded messages, and network enablement points.
- [x] Verify offline startup, disabled network behavior, and transcript visibility.

### 304 Transcript-Native Operational Visibility (deferred)
- [>] Identify routing, compaction, stop, budget, and hidden-process events not visible in transcript.
- [>] Surface those events as collapsible transcript rows without footer pollution.
- [>] Add UI/session tests proving visibility survives resume.

### 305 Principle-First Prompt Cleanup
- [x] Audit prompts for brittle examples, exception lists, and duplicated behavioral rules.
- [x] Rewrite eligible prompts into compact principle-first contracts.
- [x] Run prompt hash, scenario, and regression checks; protect `prompt_core` unless approved.

### 302 Semantic Continuity Tracking (deferred)
- [>] Trace user intent through annotation, route/formula, execution, and final answer.
- [>] Add continuity checks and failure reporting at the right orchestration boundary.
- [>] Verify with scenario tests that wrong-objective answers are detected.

### 306 Dynamic Decomposition On Weakness
- [x] Define failure signals that require decomposition rather than prompt bloat.
- [x] Add focused intermediary unit or clean-context retry behavior where needed.
- [x] Verify repeated weak-model failures produce bounded strategy shifts.

### 355 Keyword Heuristic Decomposition Audit
- [x] Inventory semantic keyword heuristics in routing, finalization, compaction, and tool choice.
- [x] Replace non-safety semantic heuristics with metadata, confidence, or focused intel units.
- [x] Keep safety/parser heuristics explicit and covered by tests.

## Wave 3: Runtime Truth, Context, And Evidence Foundations

### 338 Formal Action-Observation Event Log
- [x] Define typed action, observation, policy, lifecycle, and failure events.
- [x] Emit events from tool loop, permission, compaction, session, and finalization paths.
- [x] Verify replay/session reconstruction and visible operational rows.

### 341 Config Schema Generation And Migration
- [x] Make user-facing config TOML-first with validation and migration helpers.
- [x] Add versioned migration helpers with backup-safe behavior.
- [x] Verify invalid config paths, old fixtures, and migration failure handling.

### 343 Exact Tokenization And Model Capability Registry
- [x] Add model capability metadata and tokenizer adapter interfaces with safe fallback.
- [x] Use exact or capability-aware counts in compaction and output budgeting.
- [x] Test exact, fallback, unknown-model, and provider-limit behavior.

### 310 Deferred Pre-Turn Summary
- [x] Move summary generation to post-turn/deferred execution with effective-history integration.
- [x] Persist summaries and keep active turns grounded in recent evidence.
- [x] Verify long-session compaction and summary exclusion behavior.

### T316 Evidence-Aware Read Compaction
- [x] Add ReadFileInventory to EvidenceLedger, tracking file paths, evidence IDs, per-file summaries, and raw artifact paths.
- [x] Preserve citations, paths, and key findings in compact summaries via read_inventory_summary().
- [x] Wire inventory into auto_compact.generate_inline_summary(); summary system message now includes file inventory.
- [x] Verify tests pass (882 total, 0 failed), clean build, transcript-visible CompactBoundary rows.

### 279 Lightweight Local Auxiliary LLM Helper
- [ ] Define auxiliary model use cases that reduce main-model cognitive load.
- [ ] Implement bounded helper calls for summarization/classification where useful.
- [ ] Verify helper failures degrade to deterministic fallback behavior.

### 334 Persist Finalized Summaries As Markdown
- [ ] Define finalized summary location, naming, and metadata contract.
- [ ] Persist final summaries into session folders without duplicating transcript state.
- [ ] Verify resume/index behavior and markdown output.

### 342 Provider Fault Injection And Error Recovery Harness
- [ ] Build deterministic local provider fault fixtures for streaming and context failures.
- [ ] Add recovery assertions for timeout, malformed stream, context overflow, and provider errors.
- [ ] Verify transcript-visible recovery and no infinite retry loops.

### 345 Versioned Extension State For Sessions
- [ ] Add namespaced, versioned extension state storage for sessions.
- [ ] Provide load/save/migration helpers for optional tools and workflows.
- [ ] Verify corrupt, missing, migrated, and garbage-collected state.

## Wave 4: Action Policy And Filesystem Safety Base

### 339 Action And Tool Metadata Policy Unification
- [x] Add explicit policy metadata for every model-callable DSL action and remaining adapter.
- [x] Ensure action/tool exposure matches executor support and scheduling/permission behavior.
- [x] Verify registry, action dispatcher, executor parity, and concurrency policy tests.

### 337 File Context Tracker And Stale Read Gate (deferred)
- [>] Track read, mention, edit, and external modification state per file.
- [>] Enforce stale-read gates before write-like operations.
- [>] Verify stale, external-edit, symlink, and resume behavior.

### 361 Workspace Policy Files Ignore And Protected Paths (deferred)
- [>] Define ignore/protect policy files and matching semantics.
- [>] Apply policy consistently across read, search, edit, patch, shell, watcher, browser, and MCP.
- [>] Verify protected writes fail and ignored paths are skipped visibly.

### 340 Native Search Execution And Query Safety
- [x] Replace shell-string search execution for DSL `S`/`Y` with structured `std::process::Command` via `exec_search_dsl` helper.
- [x] Preserve timeouts (tokio::time::timeout), 50K char truncation, and evidence formatting.
- [x] Verify quote-safe, regex, path, no-match, and truncation cases (7 new tests).

## Wave 5: DSL Protocol Harness And Contract Base

### 364 DSL Protocol Coverage Matrix And Baseline Audit
- [x] Build the canonical DSL action, compatibility tool, and skill/formula inventory (DSL_PROTOCOL_MATRIX.md).
- [x] Add machine-readable matrix fixtures (protocol_matrix.toml).
- [x] Add direct `elma-cli` prompt packs (SELF_TEST_PROMPTS.md) — source-derived ground-truth answers; transcript capture deferred pending interactive sandbox.
- [x] Verify every declared action/tool, executor, and built-in skill/formula is classified.

### 365 Elma CLI DSL Protocol Self-Test Harness
- [ ] Create the disposable sandbox workspace and prompt runner/report structure.
- [ ] Support manual and scripted runs with session/transcript capture.
- [ ] Verify dry-run behavior, failure classes, and artifact paths.

### 385 Live DSL Runtime Smoke Harness And Transcript Capture
- [ ] Run `_testing_prompts/` against live `elma-cli` and capture transcript/session paths.
- [ ] Verify natural CHAT, list/read/search/shell visibility, invalid DSL repair, and evidence-grounded finalization.
- [ ] Keep advanced file write/network/browser prompts out of the basic suite until the DSL action surface supports them.

### 386 DSL Action Command Timeouts And Output Budgets
- [ ] Add explicit timeout and output-budget enforcement to `X`, `S`, and `Y`.
- [ ] Surface timeout/truncation through transcript-native tool rows and compact model observations.
- [ ] Verify huge output, timeout, no-match, command failure, and success paths.

### 387 Intel DSL Profile Repair And Live Smoke Gate
- [ ] Remove invalid-DSL fallbacks from basic live smoke for evidence assessment, turn summary, and classifier units.
- [ ] Tighten per-profile compact DSL prompts/grammar/repair without touching `src/prompt_core.rs`.
- [ ] Verify basic prompts pass with executable action rows, grounded final answers, and no model-output JSON contracts.

### 366 DSL Action Declaration Executor Contract Tests
- [x] Add parser/validator/executor parity fixtures for every executable action.
- [x] Test invalid DSL, missing fields, optional defaults, and compact observations.
- [x] Verify no callable action can normally return `Unknown action`.

## Wave 6: Executable Action Completion

### 326 Exact Edit Engine Robustness
- [x] Shared edit engine (`apply_exact_edit` in `src/dsl/safety.rs`) provides exact replacement, atomic tempfile write, and compact repair observations.
- [x] Enforce stale-read (`require_session_read_before_edit`), 8MB size limit, binary detection, UTF-8 validation, zero/multiple match errors.
- [x] Wire DSL `E` dispatch through `AgentAction::EditFile` in `src/tool_loop.rs`.
- [x] Edit snapshots via `ensure_session_edit_snapshot` pre-edit.
- [x] All verification commands pass (cargo test succeeds).

### 328 Patch Tool Multi-File Atomic Changes
- [ ] Harden parser and preserve exact patch hunk text.
- [ ] Implement transaction journal, validation, apply, and rollback executor.
- [ ] Verify success, validation failure, rollback, stale, and executor dispatch cases.

### 329 Web Fetch Tool Security-Gated HTTP
- [ ] Add explicit network enablement, fetch policy, and permission gating.
- [ ] Wire secure streaming HTTP executor with content and redirect controls.
- [ ] Verify offline default, SSRF blocks, local fixtures, truncation, and dispatch.

### 335 LSP Diagnostics And Code Intelligence Tool
- [ ] Add optional LSP diagnostics declaration and manager boundary.
- [ ] Implement fake-server-tested diagnostics execution and graceful unavailability.
- [ ] Verify no process leaks, workspace path safety, and diagnostic parsing.

### 357 Optional Browser Observation Tool
- [ ] Add disabled-by-default browser observation tool and driver abstraction.
- [ ] Reuse fetch policy and sandbox/session artifact boundaries.
- [ ] Verify fake driver, disabled state, private URL blocks, timeout cleanup, and artifacts.

### 362 Parallel Read/Search Tool Execution
- [x] Replace name-based concurrency with metadata-backed batch planning.
- [x] Integrate ordered post-processing into the tool loop.
- [x] Verify ordering, evidence, transcript, failure, and serial-barrier behavior.

## Wave 7: Execution Surfaces, Workflows, And Extensibility

### 344 Recipe And Subrecipe Workflow System
- [ ] Define external recipe schema, validation, and versioning.
- [ ] Execute bounded recipes without bloating core prompts.
- [ ] Verify loading, failure, resume, and transcript-visible stage events.

### 349 Sandboxed Execution Profile System
- [ ] Define local, restricted, and optional container execution profiles.
- [ ] Route shell/code execution through profile policy without bypassing preflight.
- [ ] Verify blocked writes, profile selection, and graceful missing container support.

### 350 Headless Event API And SDK Harness
- [ ] Add a headless session runner with JSONL/SSE-ready event output.
- [ ] Support permission callbacks and deterministic test providers.
- [ ] Verify multi-turn, denied permission, tool failure, and finalization tests.

### 358 MCP Extension Gateway With Offline Gates
- [ ] Add disabled-by-default MCP server configuration and schema import.
- [ ] Route external tools through unified metadata and permission policy.
- [ ] Verify server failure, denied external tools, and no startup network requirement.

### 359 Bounded Subagent Delegation Framework
- [ ] Define subagent scope, budget, tool permissions, and output contract.
- [ ] Implement read-only explorer delegation before write-capable delegation.
- [ ] Verify timeout, failed subagent, evidence grounding, and parent event recording.

### 336 Symbol-Aware Repo Map And Tag Cache
- [ ] Build symbol extraction/cache with token-budgeted map slices.
- [ ] Rank symbols by source evidence, recency, and task relevance.
- [ ] Verify cache invalidation, fallback parsing, and budget limits.

### 360 Persistent Project Memory And RAG Index
- [ ] Define grounded memory entry types with provenance and staleness.
- [ ] Add hybrid retrieval under strict token budget and user controls.
- [ ] Verify stale exclusion, provenance, deletion, and repeated-question improvement.

### 275 OBSERVE Step Type For Metadata-Only Inspection
- [ ] Define observe semantics distinct from read/search/action steps.
- [ ] Integrate observe into planning and transcript rows where it reduces unnecessary work.
- [ ] Verify metadata-only inspection does not mutate state or fake evidence.

### 301 Data Analysis Mode
- [ ] Define data-analysis workflow boundaries, allowed tools, and evidence expectations.
- [ ] Implement mode selection and execution path without keyword routing.
- [ ] Verify fixture-based analysis, grounding, and no unnecessary network use.

### 363 Auto Lint/Test And Verification Planner
- [ ] Detect project verification commands from manifests and changed files.
- [ ] Plan minimal verification using metadata, memory, and permissions.
- [ ] Verify command selection, permission handling, and failure reporting.

## Wave 8: UX, Diagnostics, Recovery, And Evaluation

### 346 Keybinding And Command Mode Customization
- [ ] Define keybinding config schema and defaults.
- [ ] Route keybindings to existing UI commands with conflict detection.
- [ ] Verify defaults, invalid config, and footer rule preservation.

### 347 Diagnostics Bundle And Doctor Command
- [ ] Add doctor checks and session bundle generation with redaction.
- [ ] Include config, provider, transcript, event, trace, and panic metadata.
- [ ] Verify offline bundle creation, secret redaction, and corrupt-session behavior.

### 348 Release Risk And Security Audit Gate
- [ ] Add release-risk scanner for sensitive modules and hidden/control characters.
- [ ] Integrate prompt-core, permission, shell, provider, session, and tool registry warnings.
- [ ] Verify local/offline checks and clear remediation output.

### 353 Benchmark Leaderboard And Eval Dashboard
- [ ] Normalize benchmark scenarios and scoring dimensions.
- [ ] Generate model/profile reports from headless event runs.
- [ ] Verify reliability scoring, linked transcripts, and offline reproducibility.

### 354 Terminal UI Regression Capture Harness
- [ ] Add deterministic terminal snapshots for core UI states and sizes.
- [ ] Cover streaming, operational rows, diffs, search, and footer rendering.
- [ ] Verify no overlap/truncation and footer rule enforcement.

### 351 File Watcher And AI Comment Workflow
- [ ] Add workspace watcher scoped by ignore/protect policy.
- [ ] Feed external file changes into stale context tracking and optional AI-comment triggers.
- [ ] Verify ignored paths, opt-in triggers, and transcript-visible watcher events.

### 352 Session Rewind And Checkpoint Restore UX
- [ ] Define checkpoint boundaries using events and file snapshots.
- [ ] Add list/inspect/restore UX with explicit affected files and metadata.
- [ ] Verify partial restore failure, transcript coherence, and no destructive git usage.

## Wave 9: End-To-End DSL Protocol And Skills Certification

### 367 Core DSL Actions E2E Self-Test Suite
- [x] Add prompt scenarios for `R`, `L`, `S`, `Y`, restricted `X`, `ASK`, and `DONE`.
- [x] Run prompts through the harness and classify/fix failures.
- [x] Verify transcript, evidence, session flush, and final-answer grounding.

### 368 Filesystem Mutation DSL E2E Self-Test Suite
- [x] Add sandbox prompts for exact edit, stale-read, ambiguous edit, invalid edit, and unchanged-failure behavior.
- [x] Run before/after diffs and add regressions for unsafe or failed behavior.
- [x] Verify DSL `E` and internal mutation adapters have safe success and safe failure coverage.

### 369 Network And Browser Tools E2E Security Suite
- [x] Add local HTTP/browser fixtures and default-disabled prompt tests.
- [x] Test enabled fetch/browser behavior only against controlled local fixtures.
- [x] Verify private-network, binary, redirect, and disabled-state protections.

### 370 Permission And Safe Mode DSL Policy Suite
- [x] Add prompts for safe `X`, destructive command denial, protected edits, and disabled network adapters.
- [x] Test permission policy across metadata, safe mode, and executor paths.
- [x] Verify denials are visible, recoverable, and not bypassed through other tools.

### 371 DSL Loop Evidence Transcript And Session Coherence Suite
- [x] Add prompts that exercise evidence-required answers, respond loops, and strategy changes.
- [x] Verify ledger, transcript, session artifacts, and final answer agree.
- [x] Add regressions for missing evidence, dropped tool output, or unsupported final claims.

### 372 Parallel DSL Action Ordering And Batch Execution Suite
- [x] Add prompts and fake delayed tools for parallel read/search behavior.
- [x] Verify execution can be parallel while transcript/evidence order stays deterministic.
- [x] Add regressions for serial barriers and failed sibling handling.

### 373 Skills And Formulas DSL Integration Suite
- [x] Add prompt scenarios for each built-in skill/formula path.
- [x] Verify DSL action choice matches formula expectations and user intent.
- [x] Add regressions for unnecessary tools, wrong formula, or semantic drift.

### 374 DSL Error Recovery And Small Model Self-Correction Suite
- [x] Add prompts for wrong paths, failed search, failed edit, disabled tools, and shell failures.
- [x] Verify bounded recovery through decomposition or strategy shift.
- [x] Add regressions for loops, hallucinated evidence, and repeated failed commands.

### 375 DSL Protocol Certification Gate And Release Checklist
- [x] Build the final DSL certification script and report.
- [x] Require automated tests, prompt evidence, policy classification, and parser/validator/executor parity.
- [x] Verify the gate fails on uncertified actions/tools and passes only with a complete report.

## Final Sequencing Notes

- Start with Wave 0. The DSL migration wave is now the priority path before risky new capabilities.
- Complete Tasks 376 and 377 before changing live model-output handling.
- Complete Tasks 378, 379, 380, 381, 382, and 383 before removing old model-output JSON paths.
- Complete Task 339 before Task 362 and before certifying any action/tool as production-ready.
- Complete Tasks 385 and 386 before resuming deferred advanced feature work.
- Complete Tasks 364, 365, 366, and 385 before finishing new action/tool implementations so regressions land in the harness.
- Complete Tasks 326 and 340 before their E2E certification tasks 367 and 368.
- Treat Wave 9, especially Task 375, as the final release gate for the full DSL protocol suite.
