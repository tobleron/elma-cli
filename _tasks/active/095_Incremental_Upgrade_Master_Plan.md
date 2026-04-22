# Task 095: Incremental Upgrade Master Plan

## Priority
**P0 - MASTER UPGRADE TASK**
**Created:** 2026-04-04
**Last Updated:** 2026-04-05 (Session s_1775426300 shell mutation incident)
**Status:** In Progress (Phases 1-3, 5-6 complete; Phase 4 pivoting to Shell Safety)
**Supersedes:** Task 058 (archived as completed)

## Master Plan Completion Rule
This master plan must remain in `_tasks/active/` until every phase, subtask, deferred decision, verification gate, and user sign-off item below is completed. Do not move this file to `_tasks/completed/` only because a subset of phases is done.

The master plan can be archived only after:
- [ ] Every non-postponed subtask listed under this plan is completed or deliberately superseded by a newer master plan.
- [ ] Every postponed subtask has a recorded reason, destination, and follow-up plan.
- [ ] All phase gates have passing build, test, probe, and real CLI verification evidence.
- [ ] The current `_tasks/TASKS.md` index agrees with this master plan's final status.
- [ ] The user has approved archiving this master plan.

## Continuation Checklist
- [ ] Re-read `_tasks/TASKS.md`, `AGENTS.md`, and this master plan before changing any subtask status.
- [ ] Confirm which active or pending subtasks still belong to this master plan.
- [ ] Update this checklist whenever a phase, gate, or subtask moves forward.
- [ ] Keep this file active while any child task is active, pending, blocked, or waiting for verification.
- [ ] Preserve context-management and small-local-model reliability work unless explicitly superseded.
- [ ] Run `cargo fmt --check` before final sign-off.
- [ ] Run `cargo build` before final sign-off.
- [ ] Run `cargo test` before final sign-off.
- [ ] Run relevant probes: `./probe_parsing.sh`, `./reliability_probe.sh`, `./run_intention_scenarios.sh`, and `./smoke_llamacpp.sh` where applicable.
- [ ] Run real CLI validation for user-facing behavior before final sign-off.
- [ ] Record final verification evidence in this file.
- [ ] Move this master plan to `_tasks/completed/` only after every applicable checkbox above is checked and the user approves.

## Overview

Task 058 delivered the initial stability foundation: grounded workspace discovery, stress harness scaffolding, JSON reliability, and human-style prompt handling fixes. This task drives the next phase: a disciplined incremental upgrade of Elma CLI through 8 phases with strict verification gates between each.

**Major architecture changes completed:**
- Migrated from fragile JSON-based Program orchestration to native tool calling (OpenAI `tools` API)
- Removed Maestro from tool-calling path (model plans directly, zero Maestro JSON failures)
- Tool result budget & disk persistence (50K char threshold)
- Auto-compact for context window management
- Streaming tool executor (parallel safe tools, serial shell)
- (Historical) Rose Pine theme work from older UI track; superseded for interactive parity by Task 166 Pink/Cyan token theme.
- Intel failure counter (red, end-of-session summary)
- Shell command display in trace (`→ executing shell: find . -name "*.sh"`)

**Phase 4 pivot: Shell Safety** — Session s_1775426300 exposed that the tool-calling pipeline executes destructive commands without any preflight, permission gate, or safety net. The model generated `find . -name "*.sh" | while read f; do mv "$f" "stress_testing/"; done` which would have moved 641+ files across the entire project tree. Only the missing target directory prevented disaster. This is unacceptable. Shell safety is now P0.

## Phase Structure

### Phase 1: Clean Up & Stabilize Foundation (Tasks 1-4) — DONE
1. **Task infrastructure sanitization** — archive 058, reclassify all pending tasks, correct stale blocking markers ✓
2. **Close active stress harness gaps** — built `run_stress_cli.sh` with semantic validation gates ✓
3. **Cleanup dead code** — verified already done from Task 058 work (intel.rs deleted, no unused warnings) ✓
4. **Runtime config healthcheck** — implemented `src/config_healthcheck.rs` with 8 tests, integrated into bootstrap ✓

**Gate:** `cargo build` zero warnings ✓, `cargo test` 220 passed ✓, intention scenarios 20/20 ✓, reliability probe 30/30 final_present ✓, real CLI tests — mixed results (see below)

**Real CLI Verification Results (Phase 1 Gate):**
| Scenario | Route | Result | Notes |
|----------|-------|--------|-------|
| S000A (Chat baseline) | CHAT/DECIDE | ⚠ | Over-orchestration: executed `rg --files` for a simple chat question. Final answer grounded. |
| S000B (Shell primitive) | CHAT | ⚠ | Listed files, path present, but entry point identification implicit not explicit |
| S002 (Recursive discovery) | SHELL | ⚠ | `rg --type source` unsupported; retry loop didn't change strategy; final answer reports failure |
| Combined read+summary | CHAT | ⚠ | Hallucinated entry point `scripts/run_tests.sh` — CHAT route answered without reading file |
| Sloppy greeting | CHAT | ✅ | `Hi there!` — perfect |
| Casual scoped listing | SHELL | ✅ | Bounded `ls src` workflow, real file evidence, truncated output |

**Evidence-First Routing Fix (Phase 2):**
| Prompt | Before Fix | After Fix |
|--------|-----------|-----------|
| `pwd` | `/home/user/project` (hallucinated) | `/Users/r2/elma-cli` (actual, via `pwd` execution) |
| `cwd` | `/home/user/projects` (hallucinated) | `/current working directory: /Users/r2/elma-cli` (actual) |
| "shell scripts on root?" | "No shell scripts found" (false) | `export_defaults.sh` (actual file from workspace) |

### Phase 2: Reliability Core (Tasks 5-8) — DONE
5. **Crash reporting** (065) — ✅ SessionError struct, panic hook, error.json, session status, 4 tests
6. **Output limits** (066) — deferred to Phase 5
7. **Iterative refinement** (067) — deferred to Phase 6
8. **Goal persistence** (068) — deferred to Phase 6

**Phase 2 Major Addition: Native Tool-Calling Migration**
- Replaced JSON Program orchestration with OpenAI `tools` API
- `src/tool_calling.rs` — tool registry (shell, read, search, respond)
- `src/tool_loop.rs` — continuous execution loop, 15-iteration max guard
- `orchestration_core.rs` — `run_tool_calling_pipeline()` entry point
- System prompt: explicit tool usage procedures, identity override
- Model controls pacing: calls tools until satisfied, then responds

**Phase 2 Bonus Fix: Presenter Hallucination**
| Issue | Before | After |
|-------|--------|-------|
| `ls -ltr` | Fake empty dir listing | Real file listing from shell tool |
| `pwd` | `/home/user/projects` (hallucinated) | `/Users/r2/elma-cli` (actual) |
| Identity | "I am Elma's maestro" | "I am Elma." |
| File discovery | Used `search` (wrong) → found nothing | Uses `find` via `shell` → found files |

**Gate:** `cargo build` clean ✅, `cargo test` 212 passed ✅, real CLI verification ✅

### Phase 3: Efficiency & Observability (Tasks 9-12) — DONE
9. **Tool-Calling Pipeline** — completed as part of Phase 2
10. **Context Window Management** — deferred to Phase 5
11. **Token forecasting** (088) — deferred to Phase 5
12. **Budget-aware orchestration** (089) — deferred to Phase 5

**Phase 3 Verification:** Tool-calling pipeline stable across all scenarios.

### Phase 4: Shell Safety (Tasks 16-24) — IN PROGRESS (P0)
16. **Destructive Command Detection & Preflight** (116) — P0. Classify commands by risk (safe/caution/dangerous). Preflight validates source/destination before execution. Model receives specific error guidance.
17. **Permission Gate for Destructive Commands** (117) — P0. Interactive y/N confirmation for dangerous ops. Default-deny. Session-aware approval caching.
18. **Unscoped Glob & Bulk Operation Detection** (118) — P1. Flag `find .` without `-maxdepth`, `rm *`, pipe-to-destruct patterns. Estimate match count before execution. >20 matches triggers warning.
19. **Dry-Run Mode for Destructive Commands** (119) — P1. Preview effect before committing. Model sees exact file list that would be affected.
20. **Session & Infrastructure Directory Protection** (120) — P1. Protect `sessions/`, `config/`, `_tasks/`, `src/`, `.git/` from mutation. Overrideable with explicit confirmation.
21. **Command Budget & Rate Limiting** (121) — P2. Per-session budget: dangerous ops max 5/session, caution max 20. Throttle between commands.
22. **Safe Shell Wrapper Functions** (122) — DEFERRED. Moved to `_tasks/postponed/122_Safe_Shell_Wrapper_Functions_deferred.md`. Redundant with existing preflight pipeline (5 layers deep). No safety gap.
23. **Pre-Tool Hook System** (123) — P2. Extensible hook trait. Built-in: destructive detector, path protector, unscoped glob detector, budget checker.
24. **Context Modifier System** (124) — P2. After dangerous command succeeds, temporarily lower trust. After path error, inject correction into prompt.

**Gate:** `mv nonexistent dest/` → preflight error with guidance. `find . | while read ... do mv` → flagged as bulk destructive. Session dirs protected. 0 unintended mutations in real CLI testing.

### Phase 5: Context Window & Conversation Management (Tasks 25-27) — DONE
25. **Tool Result Budget & Disk Persistence** (113) — ✅ Per-tool result threshold (50K chars), persist to disk, model sees preview + wrapper.
26. **Auto-Compact** (114) — ✅ Token tracking, inline summarization when approaching context window. Circuit breaker: max 3 failures.
27. **Streaming Tool Execution** (115) — ✅ Parallel for safe tools (read/search/respond), serial for shell. Order-preserving result merge.

**Gate:** `cargo build` clean ✅, `cargo test` 221 passed ✅, long conversations functional ✅

### Phase 6: Always-On Telegram Agent (Tasks 28-33) — POSTPONED (Tier B)
28. **Daemon Mode — HTTP Gateway** (126) — Postponed. `elma daemon` with axum server, session routing.
29. **Telegram Bot Integration** (127) — Postponed. teloxide bot, DMs route to daemon.
30. **Channel Abstraction Trait** (128) — Postponed. Generic `Channel` trait for future Discord/Slack.
31. **Background Sessions Auto-Approval** (129) — Postponed. Approval policy per channel.
32. **Always-On Service Management** (130) — Postponed. launchd/systemd integration.
33. **Telegram Session Persistence** (131) — Postponed. Survive daemon restarts.

**Estimated total:** ~1,900 lines, 5-8 days focused work. Deferred until Phase 4 complete.

### Phase 7: Lightweight UI Enhancements (Tasks 98, 99, 101, 107) — DONE
- **098** Persistent System Status Line — ✅ Context bar replaces text-based display
- **099** Context Window Usage Visualizer — ✅ Unicode progress bar (█▓▒░), color-coded
- **101** Verb-Driven Loading Spinners — ✅ Braille spinner with context-aware verbs
- **107** Visual Effort Indicator — ✅ Wall-clock effort badge, color-coded by duration

**Module design:** Three focused modules (`ui_spinner.rs`, `ui_effort.rs`, `ui_context_bar.rs`), zero cramming.

### Phase 8: Remaining Advanced Capabilities — POSTPONED
34. **Hybrid MasterPlan** (093) — Postponed
35. **Shell Mutation Snapshots** (094) — Postponed (superseded by Phase 4 safety work)
36. **Post-Tool Hook System** (125) — Completed as part of Phase 4

**Phase 7 Gate:** `cargo build` clean ✅, `cargo test` 271 passed ✅

## Phase 4 Progress

**Task 116 (Destructive Command Detection & Preflight)** — STARTING NOW

## Verification Discipline

The following tasks are formally postponed until Phases 1-7 complete:
- 073 Platform Capability Detection
- 074 Model Capability Profiles
- 075 Offline-First Network Policy
- 077 Cross-Scenario Correlation
- 078 Long-Term Tactical Memory
- 079 Predictive Failure Detection
- 080 Constraint Relaxation & Creative Problem Solving
- 081 Analogy-Based Reasoning Engine
- 082 Config Orchestrator
- 083 Document Architecture Abstractions
- 084 Refine Drag Formula Weights
- 085 Autonomous Prompt Evolution
- 086 Stress Test Sloppy Human Prompts (A/B)
- 087 Llama.cpp Runtime Token Telemetry
- 088 Objective Level Token Forecasting
- 089 Budget-Aware Orchestration
- 090 Model Behavior Mapping & Tuning Graph
- 091 Comprehensive Per-Model Tuning Lifecycle
- 092 Model Reliability Scoring & Upgrade Advice
- 093 Hybrid MasterPlan
- 094 Shell Mutation Snapshots (superseded by Phase 4)
- 098-112 UI/UX enhancement tasks (color palettes, spinners, highlighters, ratatui, etc.) — deferred until core reliability is solid

## Priority Ordering for Next Work

**Current P0: Phase 4 — Shell Safety** (tasks 116-125)

**Current Status: Phase 4 + Phase 7 Complete**

Tasks completed in order:
1. **Task 116 (Destructive Command Detection & Preflight)** — P0. ✅ DONE
2. **Task 117 (Permission Gate)** — P0. ✅ DONE
3. **Task 118 (Unscoped Glob Detection)** — P1. ✅ DONE
4. **Task 120 (Directory Protection)** — P1. ✅ DONE
5. **Task 121 (Command Budget)** — P2. ✅ DONE
6. ~~**Task 122 (Safe Shell Wrappers)**~~ — DEFERRED. Redundant with preflight.
7. **Task 123 (Pre-Tool Hooks)** — P2. ✅ DONE
8. **Task 124 (Context Modifiers)** — P2. ✅ DONE
9. **Task 125 (Post-Tool Hooks)** — P2. ✅ DONE
10. **Task 101 (Loading Spinners)** — ✅ DONE
11. **Task 107 (Effort Indicator)** — ✅ DONE
12. **Task 099 (Context Window Bar)** — ✅ DONE
13. **Task 098 (Status Line)** — ✅ DONE (merged with 099)

Rationale: The shell mutation incident (s_1775426300) proved that without preflight + permission gates, Elma is one bad model output away from catastrophic data loss. Tasks 116-117 address this directly. Tasks 118-122 build layers of defense. Tasks 123-125 create an extensible safety framework.

## Verification Discipline

Every task follows the canonical protocol:
1. `cargo build` — zero warnings
2. `cargo test` — all green
3. Relevant probes: `./probe_parsing.sh`, `./reliability_probe.sh`, `./run_intention_scenarios.sh`
4. Real `cargo run` validation for user-facing behavior
5. Sign-off before archiving

No phase advances until its gate is fully green.

## Active Sub-Tasks

- **023** — Expert Responder Transient Context (mostly done, multi-turn verification remaining)
- **064** — Real CLI Stress Harness & Reliability Gates (S000B seam narrowing in progress)

## Design Principles

- **Incremental over ambitious** — each task is surgically scoped and independently verifiable
- **Real CLI over model-path** — the canonical reliability gate is always `cargo run`, not just orchestrator checks
- **Truthful over polished** — honest failure surfacing beats masked pass labels
- **Small-model-first** — all efficiency work targets constrained local LLMs, not cloud-model assumptions
- **Principle-first** — no keyword heuristics, no word-based routing, no brittle deterministic rules
