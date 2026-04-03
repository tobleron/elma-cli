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
- [ ] **Task 056** - Cleanup dead code, legacy modules, and duplicate orchestration paths
- [ ] **Task 015** - Finish crash-reporting integration and verify fatal-path coverage
- [ ] **Task 014** - Finish hard caps and batching for read/search/summarize/shell evidence
- [ ] **Task 012** - Reframe as refinement-loop hardening, not first implementation
- [ ] **Task 013** - Reframe as full multi-turn goal execution, not just goal-state storage

### Phase B: Context Efficiency for Local Models
- [ ] **Task 020** - Hierarchical evidence compaction
- [ ] **Task 021** - Rolling conversation summary
- [ ] **Task 019** - Specialized filesystem intel for structured observation
- [ ] **Task 022** - Platform capability detection

### Phase C: Controlled Autonomy
- [ ] **Task 025** - Cross-scenario correlation
- [ ] **Task 026** - Long-term tactical memory
- [ ] **Task 029** - Predictive failure detection
- [ ] **Task 031** - Constraint relaxation and creative problem solving
- [ ] **Task 030** - Analogy-based reasoning engine

### Phase D: Self-Improvement and Maintainer Leverage
- [ ] **Task 053** - Config orchestrator tool
- [ ] **Task 054** - Document architecture abstractions
- [ ] **Task 055** - Refine drag-formula weights
- [ ] **Task 027** - Autonomous prompt evolution

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

### Task 012
Current file assumes refinement does not exist. That is outdated.
New scope: improve refinement triggers, better failure typing, and revision quality measurement.

### Task 013
Current file assumes goal state does not exist. That is outdated.
New scope: make persisted goals influence orchestration, continuation, and subgoal closure across turns.

### Task 014
Current file is directionally correct but underestimates existing truncation/compaction work.
New scope: unify hard evidence budgets across `Read`, `Search`, `Shell`, and `Summarize`.

### Task 015
Current file assumes crash reporting is absent. That is outdated.
New scope: integrate `SessionError` / panic-hook / status-file path end to end and test true failure modes.

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
- Multi-turn objectives persist and close reliably
- Failure paths always produce actionable reports
- Legacy duplication is reduced enough that architecture becomes explainable and tunable
- Advanced autonomy work begins only after the above baseline is stable

## First Recommended Active Task
Move **`_tasks/pending/056_Cleanup_Dead_Code_And_Legacy_Modules.md`** to active first.

Reason:
- it reduces architectural drag immediately
- it will expose which pathways are truly live
- it lowers the cost and risk of Tasks 015, 014, 012, and 013
