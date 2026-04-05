# Task 095: Incremental Upgrade Master Plan

## Priority
**P0 - MASTER UPGRADE TASK**
**Created:** 2026-04-04
**Status:** In Progress (Phase 1: Tasks 1-3 complete, Gate verification pending)
**Supersedes:** Task 058 (archived as completed)

## Overview

Task 058 delivered the initial stability foundation: grounded workspace discovery, stress harness scaffolding, JSON reliability, and human-style prompt handling fixes. This task drives the next phase: a disciplined incremental upgrade of Elma CLI through 4 phases with strict verification gates between each.

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

### Phase 2: Reliability Core (Tasks 5-8) — IN PROGRESS
5. **Crash reporting** (065) — ✅ SessionError struct, panic hook, error.json, session status, 4 tests
6. **Output limits** (066) — pending
7. **Iterative refinement** (067) — pending
8. **Goal persistence** (068) — pending

**Phase 2 Bonus Fix: Evidence-First Routing** — Fixed the `conservative_chat_fallback` that caused hallucination. When mode classifier is decisive (margin > 0.90, entropy < 0.10) for EXECUTE/INSPECT, it now overrides uncertain speech/workflow signals. Also prevents speech_chat_boost from eliminating non-CHAT routes when mode says action.

**Gate:** All Phase 2 tasks pass build + test + real CLI verification; no regression in Phase 1

### Phase 3: Efficiency & Observability (Tasks 9-12)
9. **Config orchestrator** (082) — `elma config` sub-command for validate/compare/visualize
10. **llama.cpp token telemetry** (087) — prompt/completion/total/remaining runtime tracking
11. **Token forecasting** (088) — objective-level cost estimation, budget envelopes
12. **Budget-aware orchestration** (089) — conservation modes, budget as live orchestration input

**Gate:** Token telemetry functional in real CLI, budget envelopes testable, config tool validates all profiles

### Phase 4: Advanced Capabilities (Tasks 13-15)
13. **Hybrid MasterPlan** (093) — generalize `masterplan → implement → verify → reply` beyond first slice
14. **Shell mutation snapshots** (094) — auto-snapshot before risky shell mutations, rollback integrity
15. **Model capability profiles** (074) — per-model adaptation for small local models

**Gate:** S005 passes without scenario-specific fallback, shell rollback works in real CLI, profiles influence runtime behavior

## Tier C (Formally Postponed)

The following tasks are formally postponed until all 4 phases complete:
- 073 Platform Capability Detection
- 075 Offline-First Network Policy
- 077 Cross-Scenario Correlation
- 078 Long-Term Tactical Memory
- 079 Predictive Failure Detection
- 080 Constraint Relaxation & Creative Problem Solving
- 081 Analogy-Based Reasoning Engine
- 083 Document Architecture Abstractions
- 084 Refine Drag Formula Weights
- 085 Autonomous Prompt Evolution
- 086 Stress Test Sloppy Human Prompts (A/B)
- 090 Model Behavior Mapping & Tuning Graph
- 091 Comprehensive Per-Model Tuning Lifecycle
- 092 Model Reliability Scoring & Upgrade Advice

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
