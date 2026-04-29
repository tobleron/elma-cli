# Master Plan

Last updated: 2026-04-29

This is the execution index for all current pending tasks. Use it to choose work in dependency order, not as a replacement for each task file. Each task file remains the source of implementation detail, verification commands, and done criteria.

## Operating Rules

- Move a task from `_tasks/pending/` to `_tasks/active/` before implementation.
- Complete the task's phases here as progress markers, but use the task file for exact scope.
- Do not mark a task complete until its own verification section passes.
- Do not modify `src/prompt_core.rs` or `TOOL_CALLING_SYSTEM_PROMPT` unless a task explicitly has user approval for that change.
- Prefer completing each wave in order. Within a wave, tasks can run in parallel only when their dependencies do not overlap.

## Wave 0: Backlog Truth And Low-Level Hygiene

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

## Wave 1: Core Behavioral Doctrine

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
- [ ] Define failure signals that require decomposition rather than prompt bloat.
- [ ] Add focused intermediary unit or clean-context retry behavior where needed.
- [ ] Verify repeated weak-model failures produce bounded strategy shifts.

### 355 Keyword Heuristic Decomposition Audit
- [ ] Inventory semantic keyword heuristics in routing, finalization, compaction, and tool choice.
- [ ] Replace non-safety semantic heuristics with metadata, confidence, or focused intel units.
- [ ] Keep safety/parser heuristics explicit and covered by tests.

## Wave 2: Runtime Truth, Context, And Evidence Foundations

### 338 Formal Action-Observation Event Log
- [ ] Define typed action, observation, policy, lifecycle, and failure events.
- [ ] Emit events from tool loop, permission, compaction, session, and finalization paths.
- [ ] Verify replay/session reconstruction and visible operational rows.

### 341 Config Schema Generation And Migration
- [ ] Generate schema artifacts for user-facing config and session settings.
- [ ] Add versioned migration helpers with backup-safe behavior.
- [ ] Verify invalid config paths, old fixtures, and migration failure handling.

### 343 Exact Tokenization And Model Capability Registry
- [ ] Add model capability metadata and tokenizer adapter interfaces with safe fallback.
- [ ] Use exact or capability-aware counts in compaction and output budgeting.
- [ ] Test exact, fallback, unknown-model, and provider-limit behavior.

### 310 Deferred Pre-Turn Summary
- [ ] Move summary generation to post-turn/deferred execution with effective-history integration.
- [ ] Persist summaries and keep active turns grounded in recent evidence.
- [ ] Verify long-session compaction and summary exclusion behavior.

### T316 Evidence-Aware Read Compaction
- [ ] Identify large read outputs that need evidence-preserving compaction.
- [ ] Preserve citations, paths, and key findings while reducing prompt payload.
- [ ] Verify final answers remain grounded after compaction.

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

## Wave 3: Tool Policy And Filesystem Safety Base

### 339 Tool Metadata Policy Unification
- [ ] Add explicit policy metadata for every declared tool.
- [ ] Ensure callable exposure matches executor support and scheduling/permission behavior.
- [ ] Verify registry, tool_search, executor parity, and concurrency policy tests.

### 337 File Context Tracker And Stale Read Gate
- [ ] Track read, mention, edit, and external modification state per file.
- [ ] Enforce stale-read gates before write-like operations.
- [ ] Verify stale, external-edit, symlink, and resume behavior.

### 361 Workspace Policy Files Ignore And Protected Paths
- [ ] Define ignore/protect policy files and matching semantics.
- [ ] Apply policy consistently across read, search, edit, patch, shell, watcher, browser, and MCP.
- [ ] Verify protected writes fail and ignored paths are skipped visibly.

### 340 Native Search Execution And Query Safety
- [ ] Replace shell-string search execution with structured command or native search APIs.
- [ ] Preserve timeouts, scoping, result limits, and evidence formatting.
- [ ] Verify quote-safe, regex, path, no-match, and truncation cases.

## Wave 4: Tool Calling Harness And Contract Base

### 364 Tool Calling Coverage Matrix And Baseline Audit
- [ ] Build the canonical tool and skill/formula inventory.
- [ ] Add direct `elma-cli` prompt packs and machine-readable matrix fixtures.
- [ ] Verify every declared tool, executor, and built-in skill/formula is classified.

### 365 Elma CLI Tool Self-Test Harness
- [ ] Create the disposable sandbox workspace and prompt runner/report structure.
- [ ] Support manual and scripted runs with session/transcript capture.
- [ ] Verify dry-run behavior, failure classes, and artifact paths.

### 366 Tool Declaration Executor Schema Contract Tests
- [ ] Add schema/executor parity fixtures for every executable and declaration-only tool.
- [ ] Test invalid JSON, missing fields, optional defaults, and output shape.
- [ ] Verify no callable tool can normally return `Unknown tool`.

## Wave 5: Executable Tool Completion

### 326 Edit Tool Robustness
- [ ] Build shared edit engine and wire both tool-calling and program-step edit paths.
- [ ] Enforce stale-read, encoding, atomic write, path, and ambiguity protections.
- [ ] Run edit engine, tool_calling, execution_steps_edit, and build verification.

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
- [ ] Replace name-based concurrency with metadata-backed batch planning.
- [ ] Integrate ordered post-processing into the tool loop.
- [ ] Verify ordering, evidence, transcript, failure, and serial-barrier behavior.

## Wave 6: Execution Surfaces, Workflows, And Extensibility

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

## Wave 7: UX, Diagnostics, Recovery, And Evaluation

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

## Wave 8: End-To-End Tool And Skills Certification

### 367 Core Tools E2E Self-Test Suite
- [ ] Add prompt scenarios for read, search, shell, respond, summary, todo, and tool_search.
- [ ] Run prompts through the harness and classify/fix failures.
- [ ] Verify transcript, evidence, session flush, and final-answer grounding.

### 368 Filesystem Mutation Tools E2E Self-Test Suite
- [ ] Add sandbox prompts for write, edit, patch, stale-read, ambiguous edit, and rollback.
- [ ] Run before/after diffs and add regressions for unsafe or failed behavior.
- [ ] Verify all mutation tools have safe success and safe failure coverage.

### 369 Network And Browser Tools E2E Security Suite
- [ ] Add local HTTP/browser fixtures and default-disabled prompt tests.
- [ ] Test enabled fetch/browser behavior only against controlled local fixtures.
- [ ] Verify private-network, binary, redirect, and disabled-state protections.

### 370 Permission And Safe Mode Tool Policy Suite
- [ ] Add prompts for safe shell, destructive shell, protected edits, and disabled network tools.
- [ ] Test permission policy across metadata, safe mode, and executor paths.
- [ ] Verify denials are visible, recoverable, and not bypassed through other tools.

### 371 Tool Loop Evidence Transcript And Session Coherence Suite
- [ ] Add prompts that exercise evidence-required answers, respond loops, and strategy changes.
- [ ] Verify ledger, transcript, session artifacts, and final answer agree.
- [ ] Add regressions for missing evidence, dropped tool output, or unsupported final claims.

### 372 Parallel Tool Ordering And Batch Execution Suite
- [ ] Add prompts and fake delayed tools for parallel read/search behavior.
- [ ] Verify execution can be parallel while transcript/evidence order stays deterministic.
- [ ] Add regressions for serial barriers and failed sibling handling.

### 373 Skills And Formulas Tool Integration Suite
- [ ] Add prompt scenarios for each built-in skill/formula path.
- [ ] Verify tool choice matches formula expectations and user intent.
- [ ] Add regressions for unnecessary tools, wrong formula, or semantic drift.

### 374 Tool Error Recovery And Small Model Self-Correction Suite
- [ ] Add prompts for wrong paths, failed search, failed edit, disabled tools, and shell failures.
- [ ] Verify bounded recovery through decomposition or strategy shift.
- [ ] Add regressions for loops, hallucinated evidence, and repeated failed commands.

### 375 Tool Calling Certification Gate And Release Checklist
- [ ] Build the final certification script and report.
- [ ] Require automated tests, prompt evidence, policy classification, and executor parity.
- [ ] Verify the gate fails on uncertified tools and passes only with a complete report.

## Final Sequencing Notes

- Start with Wave 0 and Wave 1 before implementing risky tools.
- Complete Task 339 before Task 362 and before certifying any tool as production-ready.
- Complete Tasks 364, 365, and 366 before finishing new tool implementations so regressions land in the harness.
- Complete Tasks 326, 328, and 329 before their E2E certification tasks 368 and 369.
- Treat Wave 8, especially Task 375, as the final release gate for the full tool-calling suite.
