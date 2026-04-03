# Task 058: Incremental Stability Master Plan

## Priority
**P0 - MASTER STABILIZATION TASK**
**Created:** 2026-04-03
**Basis:** Repository-wide architectural audit, code inspection, task audit, `cargo build`, `cargo test`, `./run_intention_scenarios.sh`, and `./reliability_probe.sh`

## Status
**ACTIVE**

## Objective
Turn Elma from a promising architecture with several strong subsystems into a consistently stable, truth-grounded, incrementally autonomous local-LLM agent.

## Current Audit Snapshot

### Verified Working Foundations
- `cargo build` passes
- `cargo test` passes (`143` tests)
- `./run_intention_scenarios.sh` completes across all current intention scenarios
- `./reliability_probe.sh` shows stable final-answer separation on the active local server
- Execution ladder, reflection, intel trait, goal persistence scaffolding, crash-reporting scaffolding, read/search steps, workspace tree, and reviewer/sufficiency narrative paths all exist in live code

### Main Reality Check
Elma is **on the right track**, but the repo is now in a **mid-transition state**:
- the architecture is significantly more advanced than some task docs still claim
- the philosophy is visible in the runtime
- several anti-philosophy shortcuts still exist in formula selection, drift detection, prompt shape, and fallback logic
- multiple pending tasks should be reframed as **hardening/completion** tasks, not greenfield work

## Strength / Failure Node Count

### Success Nodes: 12
1. Modularized runtime compared with old monolithic `main.rs`
2. Typed step system with `Read` and `Search` added
3. Execution ladder implemented and tested
4. Intel trait architecture implemented
5. JSON parsing / repair / fallback infrastructure implemented
6. Reflection integrated into main chat flow
7. Reviewer / sufficiency / narrative evidence pipeline implemented
8. Goal-state persistence scaffolding implemented
9. Crash-reporting scaffolding implemented
10. Tool discovery and workspace tree context exist
11. Scenario and probe assets are substantial and useful
12. Build and test baseline is currently healthy

### Failure Nodes To Control: 10
1. Keyword-driven logic still exists in runtime-critical paths
2. Several prompts still use example-heavy or non-standard output contracts
3. Pending task docs are stale and misstate implementation reality
4. Very large modules still concentrate risk (`src/intel_units.rs`, `src/json_error_handler.rs`, `src/program_policy.rs`, `src/defaults_evidence.rs`, `src/types_core.rs`)
5. Some guardrails use brittle lexical heuristics instead of evidence or confidence
6. Long-context robustness is still incomplete (read/search/summarize/output caps, rolling summary, compaction)
7. Goal persistence exists structurally but is not yet a complete multi-turn planning loop
8. Crash reporting exists structurally but needs full integration and verification coverage
9. Architecture duplication / legacy compatibility layers still increase cognitive drag
10. Advanced autonomy tasks are queued before full stabilization is finished

## Reordered Execution Plan

Use this order as the authoritative execution sequence. Do **not** prioritize by old filename numbers.

### Phase A: Truthfulness and Reliability Closure
- [ ] **Task 061** - Role-based temperature and retry strategy calibration
- [ ] **Task 062** - Final answer presentation and formatting reliability
- [ ] **Task 063** - Real CLI stress harness and reliability gates
- [ ] **Task 094** - Snapshot coverage for shell mutations and rollback integrity
- [ ] **Task 093** - Hybrid masterplan and phase-implementation reliability
- [ ] **Task 064** - Cleanup dead code, legacy modules, and duplicate orchestration paths
- [ ] **Task 065** - Finish crash-reporting integration and verify fatal-path coverage
- [ ] **Task 066** - Finish hard caps and batching for read/search/summarize/shell evidence
- [ ] **Task 067** - Reframe as refinement-loop hardening, not first implementation
- [ ] **Task 068** - Reframe as full multi-turn goal execution, not just goal-state storage
- [ ] **Task 069** - Runtime profile validation and config healthcheck

### Phase B: Context Efficiency for Local Models
- [ ] **Task 087** - llama.cpp runtime token telemetry
- [ ] **Task 088** - Objective-level token forecasting and budget envelopes
- [ ] **Task 089** - Budget-aware orchestration and aggressive context conservation
- [ ] **Task 070** - Hierarchical evidence compaction
- [ ] **Task 071** - Rolling conversation summary
- [ ] **Task 072** - Specialized filesystem intel for structured observation
- [ ] **Task 073** - Platform capability detection
- [ ] **Task 074** - Model capability profiles for local LLMs
- [ ] **Task 075** - Offline-first network policy and web fallbacks
- [ ] **Task 076** - Trace observability and session review tools

### Phase C: Controlled Autonomy
- [ ] **Task 077** - Cross-scenario correlation
- [ ] **Task 078** - Long-term tactical memory
- [ ] **Task 079** - Predictive failure detection
- [ ] **Task 080** - Constraint relaxation and creative problem solving
- [ ] **Task 081** - Analogy-based reasoning engine

### Phase D: Self-Improvement and Maintainer Leverage
- [ ] **Task 082** - Config orchestrator tool
- [ ] **Task 083** - Document architecture abstractions
- [ ] **Task 084** - Refine drag-formula weights
- [ ] **Task 085** - Autonomous prompt evolution

### Phase E: Late-Stage Realism Validation
- [ ] **Task 086** - Stress-test sloppy human prompts and intent-helper A/B behavior

### Phase F: Late-Stage Cross-Model Adaptation
- [ ] **Task 090** - Model behavior mapping and tuning graph
- [ ] **Task 091** - Comprehensive per-model tuning lifecycle
- [ ] **Task 092** - Model reliability scoring and upgrade advice

## Current Master Checklist

### Completed In This Masterplan Thread
- [x] Repository-wide architectural audit completed
- [x] Success-node / failure-node assessment completed
- [x] New stabilization master plan created
- [x] Canonical prompt registry introduced in code
- [x] Startup prompt enforcement added for managed intel profiles
- [x] Key classifier grammars aligned with compact numbered-choice JSON
- [x] `status_message_generator` moved onto the managed canonical profile path
- [x] Inline shell-step prompt bypass removed from execution runtime
- [x] First trait-migration slice completed for the active `intel.rs` production path
- [x] Live runtime callers migrated off `intel.rs` compatibility helpers
- [x] Duplicate `src/intel.rs` module retired
- [x] Trait layer cleanup started after legacy intel removal
- [x] Repeated intel-unit request construction consolidated into shared trait helpers
- [x] Repeated intel-unit execute boilerplate consolidated into shared trait helpers
- [x] Remaining special-case complexity intel execution normalized onto shared traced helpers
- [x] Live ladder path now preserves `workflow_plan` instead of dropping it
- [x] Evidence/action assessment restored as independent ladder signals
- [x] Early planning intel units switched onto shared narrative builders
- [x] Evidence-mode lexical short-circuit removed so the unit owns the decision
- [x] Stress-testing prompts S001-S008 rewritten for safer incremental escalation
- [x] Stress-testing prompts sandboxed to `_stress_testing/` targets
- [x] Stress-test runners now reject prompts that do not anchor to `_stress_testing/`
- [x] Stress-testing sandbox contract documented in `_stress_testing/README.md`

### Immediate Next Execution Targets
- [x] Archive finished active task `006_Extend_Narrative_To_All_Intel_Units`
- [x] Move this master plan into `_tasks/active/`
- [x] Start **Task 056** as the first implementation task under this master plan
- [x] Continue **Task 056** by simplifying remaining trait-unit duplication and consolidating repeated request builders
- [x] Continue **Task 056** by extracting the next shared intel-unit schema/request helpers
- [x] Continue **Task 056** by normalizing the remaining special-case intel execution paths
- [x] Verify orchestration-sequence and narrative gaps before stress testing
- [x] Refine startup workspace context so routing/planning gets concise OS/shell/tool facts
- [x] Reduce live classifier transport bloat by removing unused router logprob payloads
- [x] Stabilize the first blocked sandbox CLI probe (`S000E`) through real end-to-end completion
- [x] Stabilize `S000F` through grounded select/select/reply completion in the real CLI
- [x] Close the evidence-free `DECIDE` reliability hole exposed by `S000H`
- [x] Identify gaps in pending-task coverage for premium-quality local-model reliability
- [x] Create new missing pending tasks for retry calibration, presentation reliability, CLI stress gates, config healthcheck, local-model capability profiles, offline-first network policy, and session review tooling
- [x] Renumber the pending task queue to reflect the actual stabilization path
- [x] Add advanced llama.cpp-first runtime token budgeting tasks for later-phase local-model efficiency work
- [x] Add late-stage cross-model tuning, scoring, and upgrade-advice roadmap tasks
- [x] Identify and track the hybrid masterplan-plus-Phase-1 implementation capability gap as Task 093
- [x] Identify and track the shell-mutation auto-snapshot gap as Task 094
- [x] Land the first working hybrid masterplan fallback slice for `S005` in the real CLI
- [x] Land a working plan-level architecture-audit fallback for `S006` in the real CLI
- [x] Land a working bounded subset-refactor fallback for `S007` in the real CLI
- [x] Land a working documentation-audit endurance fallback for `S008` in the real CLI
- [ ] Start the sandboxed CLI stress ladder against `_stress_testing` prompts
- [x] Verify stress-suite runner behavior against the rewritten `_stress_testing` prompts
- [x] Add explicit sandbox expectations to the stress-test runner and/or docs if needed
- [ ] Run the CLI stress ladder incrementally against the sandbox-anchored prompts

### Done Criteria For This Masterplan
- [ ] Phase A complete
- [ ] Phase B complete
- [ ] Phase C complete
- [ ] Phase D complete
- [ ] `_stress_testing` ladder passes at the intended incremental escalation boundaries
- [ ] Elma remains confined to stress sandboxes during stress-test execution

## Required Reframing of Existing Pending Tasks

### Task 067
Current file assumes refinement does not exist. That is outdated.
New scope: improve refinement triggers, better failure typing, and revision quality measurement.

### Task 068
Current file assumes goal state does not exist. That is outdated.
New scope: make persisted goals influence orchestration, continuation, and subgoal closure across turns.

### Task 066
Current file is directionally correct but underestimates existing truncation/compaction work.
New scope: unify hard evidence budgets across `Read`, `Search`, `Shell`, and `Summarize`.

### Task 065
Current file assumes crash reporting is absent. That is outdated.
New scope: integrate `SessionError` / panic-hook / status-file path end to end and test true failure modes.

### Task 061
New scope: unify role temperatures, strategy-chain behavior, retry escalation, and local-model-safe stochasticity envelopes under one calibrated policy.

### Task 062
New scope: prevent last-mile presenter/formatter corruption so grounded answers stay plain, direct, and terminal-appropriate.

### Task 063
New scope: make `cargo run` the authoritative stress-validation path, not only orchestrator-model approximations.

### Task 094
New scope: extend automatic recovery snapshots beyond structured `Edit` steps so shell-based workspace mutations are also rollback-safe.

### Task 069
New scope: validate profiles, grammars, prompt sync, and runtime config compatibility at startup.

### Task 074
New scope: define capability-aware runtime behavior for local small LLMs without forking the core prompt contract.

### Task 075
New scope: codify offline-first behavior with honest web fallback policy.

### Task 076
New scope: make session failures easier to inspect and explain from local trace artifacts.

### Task 087
New scope: capture authoritative runtime token usage and remaining-context telemetry from llama.cpp-style local endpoints.

### Task 088
New scope: forecast token cost for the active objective and produce budget envelopes before long workflows spend the context recklessly.

### Task 089
New scope: make budget-aware orchestration and aggressive context conservation a real runtime behavior, especially for 1B/3B local-model deployments.

### Task 090
New scope: map each model's behavioral deviations against Elma's benchmark expectations and record successful mitigations without philosophy-breaking prompt hacks.

### Task 091
New scope: run a comprehensive per-model tuning lifecycle that exhausts safe runtime adjustments to achieve best-known Elma compliance.

### Task 092
New scope: produce a final tuned reliability score and user-facing advice about whether the model is truly suitable for Elma workloads.

## Architectural Guardrails For All Future Work
- Remove runtime keyword matching where confidence or evidence-based judgment should decide
- Prefer principle-first prompts with minimal examples
- Keep intel units narrow and measurable
- Prioritize bounded evidence flow over raw-output expansion
- Preserve backward compatibility only when it reduces real migration risk
- Measure progress with scenarios, probes, retries, and grounded-failure rate, not only unit tests

## Success Criteria
- Routing / planning / verification behave consistently without brittle lexical shortcuts
- Large-output tasks do not overflow context or silently degrade quality
- Runtime token budgeting helps Elma preserve quality on constrained local models before overflow pressure becomes critical
- Multi-turn objectives persist and close reliably
- Failure paths always produce actionable reports
- Legacy duplication is reduced enough that architecture becomes explainable and tunable
- Advanced autonomy work begins only after the above baseline is stable
- Cross-model tuning and scoring begin only after Elma's own single-model reliability baseline is strong enough to serve as a canonical benchmark

## First Recommended Pending Follow-Up
Move **`_tasks/pending/061_Role_Based_Temperature_And_Retry_Strategy_Calibration.md`** to active after the current troubleshooting thread closes.

Reason:
- it closes a live reliability gap already visible in retries and strategy reuse
- it gives the local-model stack a principled role-based stochasticity policy
- it reduces wasted loops before the broader CLI stress harness work expands
