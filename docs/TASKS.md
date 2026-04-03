# Task Priority Master List

**Last Updated:** 2026-04-03  
**Status:** Reorganized by 4 foundational pillars (P0-1 through P0-4)

> **See [ARCHITECTURE.md](./ARCHITECTURE.md)** for comprehensive documentation including design philosophy, GBNF grammar integration, JSON reliability pipeline, and full implementation details.
> **See [IMPLEMENTATION_NOTES.md](./IMPLEMENTATION_NOTES.md)** for recent progress, troubleshooting sessions, and current state.

---

## 🎯 EXECUTION SUMMARY

### Current State
- **Active Tasks:** 1 (Task 001 - JSON Reliability Masterplan, Phase 2 Complete)
- **Pending Tasks:** 20 (organized by pillar priority)
- **Completed Tasks:** 37+ (moved to `_tasks/completed/`)
- **Postponed Tasks:** 15 (blocked until P0-1 through P0-4 complete)

### Priority Sequence (Strict)
**PHASE 1: JSON Reliability (Weeks 1-4)** → **PHASE 2: Context Narrative (Weeks 5-7)** → **PHASE 3: Workflow Sequence (Weeks 8-10)** → **PHASE 4: Reliability Tasks (Weeks 11-16)**

**DO NOT START PHASE 4 UNTIL PHASES 1-3 ARE COMPLETE.**

---

## 📊 PILLAR STRUCTURE

| Pillar | Focus | Tasks | Status |
|--------|-------|-------|--------|
| **P0-1** | JSON Reliability (100% Guaranteed Output) | 4 tasks (001-004) | 🔴 All NEW/IN PROGRESS |
| **P0-2** | Context Narrative (Right Context for Each Unit) | 3 tasks (005-007) | 🔴 2 NEW, 1 renumbered |
| **P0-3** | Workflow Sequence (Optimal Configuration) | 4 tasks (008-011) | 🔴 All NEW |
| **P0-4** | Reliability Tasks (Error Handling, Persistence) | 7 tasks (012-018) | 🟡 All renumbered from existing |

---

## 📋 COMPLETE TASK LIST BY PILLAR

### **PILLAR 1: JSON RELIABILITY (100% Guaranteed Output)**

#### Task 001: GBNF Grammar Enforcement for All Intel Units
**Priority:** P0-1.1 (CRITICAL - FIRST)  
**Status:** NOT STARTED → Phase 2 Complete  
**Objective:** Enforce 100% valid JSON output from all intel units using GBNF grammars.

**Technical Tasks:**
- [x] Audit all 78 default profiles for JSON output requirements
- [x] Create GBNF grammar files for 4 critical profiles (router, complexity, evidence_needs, action_needs)
- [x] Update `Profile` struct to include optional `grammar` field
- [x] Modify `chat_once()` to send grammar with request
- [x] Test grammar enforcement for all intel units

**Files Created:**
- `config/grammars/` directory (4 GBNF files)
- `src/json_grammar.rs` - Grammar loading and injection
- `config/grammar_mapping.toml` - Profile-to-grammar mapping

**Acceptance Criteria:**
- ✅ All intel unit responses are 100% valid JSON (zero parse failures for enabled profiles)
- ✅ Grammar files documented and versioned
- ✅ No performance degradation (>95% of current speed)

**Dependencies:** None (foundational)

---

#### Task 002: Robust JSON Parser with Auto-Repair
**Priority:** P0-1.2 (CRITICAL)  
**Status:** NOT STARTED  
**Objective:** Replace current JSON parsing with robust parser that auto-repairs common errors.

**Technical Tasks:**
- [ ] Audit current `json_parser.rs` (531 LOC) for failure modes
- [ ] Implement repair strategies:
  - Missing closing braces/brackets
  - Unescaped quotes in strings
  - Trailing commas
  - Single quotes → double quotes
  - Comments removal
- [ ] Add `jsonrepair-rs` integration (already in Cargo.toml)
- [ ] Implement multi-pass parsing pipeline
- [ ] Log all repairs for analysis

**Files to Modify:**
- `src/json_parser.rs` - Complete rewrite
- `src/json_error_handler.rs` - Integrate with circuit breaker

**Acceptance Criteria:**
- ✅ 99%+ parse success rate (currently ~85-90%)
- ✅ All repairs logged with before/after
- ✅ Fallback to regex extraction if all else fails

**Dependencies:** Task 001 (GBNF reduces repair load)

---

#### Task 003: JSON Repair Intel Unit (Advanced)
**Priority:** P0-1.3 (CRITICAL - ADVANCED FEATURE)  
**Status:** NOT STARTED  
**Objective:** Dedicated intel unit that repairs malformed JSON using model intelligence.

**Technical Tasks:**
- [ ] Create `src/intel_units/json_repair_unit.rs`
- [ ] Design prompt for JSON repair (principle-first style)
- [ ] Create `config/defaults/json_repair_intel.toml` profile
- [ ] Integrate into parsing pipeline as step 3 (after auto-repair)

**Files to Create:**
- `src/intel_units/json_repair_unit.rs`
- `config/defaults/json_repair_intel.toml`

**Acceptance Criteria:**
- ✅ Repairs JSON that auto-repair cannot fix
- ✅ Preserves semantic meaning (not just syntax)
- ✅ Adds <500ms latency

**Dependencies:** Task 002 (auto-repair first, intel unit second)

---

#### Task 004: Schema Validation for All Intel Outputs
**Priority:** P0-1.4 (CRITICAL)  
**Status:** NOT STARTED  
**Objective:** Validate all intel unit outputs against schemas before use.

**Technical Tasks:**
- [ ] Define schemas for all intel output types
- [ ] Implement validation in `json_error_handler.rs`
- [ ] Add schema to each intel unit profile
- [ ] Reject invalid outputs and trigger repair

**Files to Modify:**
- `src/json_error_handler.rs` - Add schema validation
- `src/types_api.rs` - Add schema definitions

**Acceptance Criteria:**
- ✅ All intel outputs validated before use
- ✅ Invalid outputs trigger repair, not crash

**Dependencies:** Task 003 (repair pipeline must exist first)

---

### **PILLAR 2: CONTEXT NARRATIVE (Right Context for Each Unit)**

#### Task 005: Narrative Format Specification
**Priority:** P0-2.1 (CRITICAL)  
**Status:** NOT STARTED  
**Objective:** Define narrative format standards for all workflow units.

**Technical Tasks:**
- [ ] Audit all 78 default profiles for current input format
- [ ] Classify units by narrative needs (classification, planning, execution, verification, repair)
- [ ] Design narrative template for each class
- [ ] Document narrative principles (plain text, no JSON blobs, ultra-concise)

**Files to Create:**
- `docs/NARRATIVE_STANDARD.md`
- `src/intel_narrative.rs` - Expand with all templates

**Acceptance Criteria:**
- ✅ All 78 profiles classified by narrative type
- ✅ Narrative templates documented
- ✅ Token count reduced by 30% vs. JSON format

**Dependencies:** None (can parallelize with Task 001-004)

---

#### Task 006: Extend Intel Narrative Module to All Units
**Priority:** P0-2.2 (CRITICAL)  
**Status:** PARTIAL (Phase 1 done for critic)  
**Objective:** Implement narrative builders for ALL intel units (not just critic).

**Technical Tasks:**
- [ ] `build_complexity_narrative()` - Already exists, verify format
- [ ] `build_evidence_needs_narrative()` - Already exists, verify format
- [ ] `build_action_needs_narrative()` - Already exists, verify format
- [ ] `build_route_narrative()` - NEW for routing units
- [ ] `build_workflow_planner_narrative()` - NEW
- [ ] `build_tooler_narrative()` - NEW

**Files to Modify:**
- `src/intel_narrative.rs` - Add all narrative builders

**Acceptance Criteria:**
- ✅ All intel units use narrative format
- ✅ No JSON entropy/distribution in intel input (except where needed)

**Dependencies:** Task 005 (narrative spec first)

---

#### Task 007: Context Boundary Enforcement
**Priority:** P0-2.3 (CRITICAL)  
**Status:** NOT STARTED  
**Objective:** Ensure each unit receives ONLY the context it needs (no over-sharing).

**Technical Tasks:**
- [ ] Audit current context sharing across units
- [ ] Define context boundaries per unit class (Minimal, Classification, Planning, Execution, Verification)
- [ ] Implement context builder function
- [ ] Update all intel unit calls to use appropriate boundary

**Files to Create:**
- `src/context_boundary.rs`

**Acceptance Criteria:**
- ✅ Each unit receives minimal sufficient context
- ✅ Token usage reduced by 20%+ vs. current

**Dependencies:** Task 006 (narrative format first)

---

### **PILLAR 3: WORKFLOW SEQUENCE OPTIMIZATION**

#### Task 008: Workflow Sequence Analysis
**Priority:** P0-3.1 (CRITICAL)  
**Status:** NOT STARTED  
**Objective:** Analyze and document optimal workflow sequence.

**Technical Tasks:**
- [ ] Document CURRENT sequence with full dependency graph
- [ ] Identify sequence problems and optimization opportunities
- [ ] Propose OPTIMAL sequence with justification
- [ ] Create performance model (which sequence minimizes tokens/latency)

**Files to Create:**
- `docs/WORKFLOW_SEQUENCE.md`

**Acceptance Criteria:**
- ✅ Current sequence fully documented
- ✅ Dependency graph complete
- ✅ Optimal sequence proposed with justification

**Dependencies:** None (analysis task)

---

#### Task 009: Workflow Sequence Reordering
**Priority:** P0-3.2 (CRITICAL)  
**Status:** NOT STARTED  
**Objective:** Implement optimal workflow sequence from Task 008.

**Technical Tasks:**
- [ ] Refactor `app_chat_core.rs::run_chat_loop()` with optimal sequence
- [ ] Move ladder assessment before complexity
- [ ] Make workflow planner conditional (only if ladder >= Task)
- [ ] Add sequence validation function

**Files to Modify:**
- `src/app_chat_core.rs` - Main sequence
- `src/execution_ladder.rs` - Move earlier in sequence

**Acceptance Criteria:**
- ✅ Optimal sequence implemented
- ✅ All scenario tests passing

**Dependencies:** Task 008 (analysis first)

---

#### Task 010: Conditional Workflow Planning
**Priority:** P0-3.3 (CRITICAL)  
**Status:** NOT STARTED  
**Objective:** Only run workflow planner when ladder >= Task (skip for Action level).

**Technical Tasks:**
- [ ] Modify `build_program()` to check ladder level
- [ ] Create `build_direct_program()` for Action-level tasks
- [ ] Update formula definitions to include ladder level

**Files to Create:**
- `src/program_building_direct.rs` - Direct program builder

**Acceptance Criteria:**
- ✅ Action-level tasks skip workflow planner
- ✅ Token usage reduced by 30% for Action tasks

**Dependencies:** Task 009 (sequence reordering first)

---

#### Task 011: Parallel Intel Unit Execution
**Priority:** P0-3.4 (OPTIMIZATION)  
**Status:** NOT STARTED  
**Objective:** Run independent intel units in parallel to reduce latency.

**Technical Tasks:**
- [ ] Identify parallelizable units (complexity + evidence + action, or logical + efficiency + risk reviewers)
- [ ] Implement parallel execution with `tokio::join!`
- [ ] Add parallel execution config (max_concurrent, timeout_ms)

**Files to Create:**
- `src/intel_parallel.rs` - Parallel execution orchestration

**Acceptance Criteria:**
- ✅ Independent units run in parallel
- ✅ Latency reduced by 40%+ for classification phase

**Dependencies:** Task 009 (sequence must be stable first)

---

### **PILLAR 4: RELIABILITY TASKS (Previously Priority, Now After P0-1,2,3)**

#### Task 012: Comprehensive Error Reporting
**Priority:** P0-4.1 (CRITICAL - but blocked on P0-1,2,3)  
**Status:** NOT STARTED  
**Objective:** Every session produces actionable error reports or success markers.

**Technical Tasks:**
- [ ] Create `src/error_report.rs` with `SessionError` struct
- [ ] Install panic hook in `main.rs`
- [ ] Write `error.json` on any fatal error
- [ ] Add session status tracking (`session_status.json`)

**Acceptance Criteria:**
- ✅ No session ends without `error.json` or success marker
- ✅ Panic writes stack trace to log

**Dependencies:** P0-1,2,3 must be complete first

---

#### Task 013: Multi-Turn Goal Persistence
**Priority:** P0-4.2 (CRITICAL - but blocked on P0-1,2,3)  
**Status:** NOT STARTED  
**Objective:** GoalState persists across session restarts.

**Technical Tasks:**
- [ ] Add `goals_dir` to `SessionPaths`
- [ ] Serialize `GoalState` to `active_goal.json`
- [ ] Load `GoalState` on session start

**Acceptance Criteria:**
- ✅ GoalState persists across session restarts
- ✅ User can resume multi-turn workflows

**Dependencies:** P0-1,2,3 must be complete first

---

#### Task 014: Ground Critics in Actual Output
**Priority:** P0-4.3 (CRITICAL - but blocked on P0-1,2,3)  
**Status:** NOT STARTED  
**Objective:** Critics verify grounding in actual step results, not hallucinate failures.

**Technical Tasks:**
- [ ] Fix `critic.toml` prompt (principle-first, grounded)
- [ ] Add `verify_grounding()` function
- [ ] Reject critic verdicts without evidence

**Acceptance Criteria:**
- ✅ Critics catch real failures only
- ✅ Hallucination rate <5%

**Dependencies:** P0-1,2,3 must be complete first (narrative format helps)

---

#### Task 015: Enable Reflection for ALL Tasks
**Priority:** P0-4.4 (CRITICAL - but blocked on P0-1,2,3)  
**Status:** NOT STARTED  
**Objective:** Reflection runs for all tasks (not just non-DIRECT).

**Technical Tasks:**
- [ ] Remove DIRECT skip condition in `app_chat_core.rs`
- [ ] Add lightweight reflection mode for DIRECT tasks

**Acceptance Criteria:**
- ✅ Reflection runs for all tasks
- ✅ DIRECT tasks get lightweight reflection (1-2 questions)

**Dependencies:** P0-1,2,3 must be complete first

---

#### Task 016: Workspace Context Optimization
**Priority:** P0-4.5 (HIGH - but blocked on P0-1,2,3)  
**Status:** NOT STARTED  
**Objective:** Tree view with `ignore` crate, reduce verbosity.

**Technical Tasks:**
- [ ] Implement tree view in `workspace_tree.rs`
- [ ] Add file importance heuristics
- [ ] Limit recent files list

**Acceptance Criteria:**
- ✅ Token usage reduced by 30%
- ✅ Model accuracy maintained

**Dependencies:** P0-1,2,3 must be complete first

---

#### Task 017: Evidence Compaction
**Priority:** P0-4.6 (HIGH - but blocked on P0-1,2,3)  
**Status:** NOT STARTED  
**Objective:** Hierarchical evidence compaction to reduce token bloat.

**Technical Tasks:**
- [ ] Create `src/evidence_compaction.rs`
- [ ] Rank evidence by importance
- [ ] Summarize low-importance items

**Acceptance Criteria:**
- ✅ Token usage reduced by 40% for long sessions
- ✅ Evidence integrity maintained

**Dependencies:** P0-1,2,3 must be complete first

---

#### Task 018: Rolling Conversation Summary
**Priority:** P0-4.7 (HIGH - but blocked on P0-1,2,3)  
**Status:** NOT STARTED  
**Objective:** Summarize old messages to prevent context bloat.

**Technical Tasks:**
- [ ] Create `src/session_summary.rs`
- [ ] Summarize every N turns
- [ ] Keep last M messages verbatim

**Acceptance Criteria:**
- ✅ Context stays under token limit
- ✅ Objective preserved across turns

**Dependencies:** P0-1,2,3 must be complete first

---

## 📊 OLD TASK RENUMBERING MAP

See [ARCHITECTURE.md](./ARCHITECTURE.md#swot-analysis-summary) for the complete SWOT analysis and strategic recommendations.

| Old Task # | New Task # | Name | Status |
|------------|------------|------|--------|
| 047 | 006 | Extend Narrative To All Intel Units | Renumbered + Expanded |
| 016 | 012 | Improve Crash Reporting | Renumbered |
| 014 | 013 | Multi-Turn Goal Persistence | Renumbered |
| 013 | (merged) | Iterative Program Refinement | Merged into P0-3 tasks |
| 015 | (merged) | Limit Summarize Output | Merged into Task 017 |
| 044 | 016 | Workspace Context Optimization | Renumbered |
| 020 | 017 | Hierarchical Evidence Compaction | Renumbered |
| 021 | 018 | Rolling Conversation Summary | Renumbered |

---

## 🎯 METRICS TARGETS

### After completing P0-1 (JSON Reliability):
- **+30% accuracy** (reflection on all tasks)
- **+25% routing** (correct speech act classification)
- **Zero crashes** (JSON fallback everywhere)

### After completing P0-1 through P0-4:
- **+20% formula accuracy** (reply family revised)
- **+30% plan quality** (plan family revised)
- **Better modularity** (classification decoupled)

---

## ✅ NEXT ACTION

See [IMPLEMENTATION_NOTES.md](./IMPLEMENTATION_NOTES.md) for recent progress and current state.

**Start Task 001: GBNF Grammar Enforcement** (see [TASKS.md](./TASKS.md#task-001-gbnf-grammar-enforcement-for-all-intel-units))

This is the absolute foundation. Without 100% valid JSON output, everything else is fragile.

**First steps:**
1. Read all 78 default profiles to identify JSON output types
2. Create `config/grammars/` directory
3. Write first GBNF grammar (choice_1of2)
4. Test with router.toml

---

## 📚 Quick Links

- **[ARCHITECTURE.md](./ARCHITECTURE.md)** - Comprehensive reference documentation
- **[TASKS.md](./TASKS.md)** - Complete task list by pillar
- **[IMPLEMENTATION_NOTES.md](./IMPLEMENTATION_NOTES.md)** - Recent progress & troubleshooting
- **[INTEL_UNIT_STANDARD.md](./INTEL_UNIT_STANDARD.md)** - Intel unit output format standard
- **[JSON_TEMPERATURE_TUNING.md](./JSON_TEMPERATURE_TUNING.md)** - Temperature tuning system

---

## 🚀 Essential Commands

### Development
```bash
cargo build
cargo run -- [args]
cargo test
cargo fmt
```

### Testing & Probing
```bash
# Run unit tests
cargo test

# Run behavioral probes
./probe_parsing.sh
./reliability_probe.sh
./run_intention_scenarios.sh
./smoke_llamacpp.sh

# Run stress tests
./run_stress_tests_cli.sh
```

### Architecture Analysis
```bash
# Run the de-bloating analyzer
cd _dev-system/analyzer && cargo run
```

### Configuration Management
```bash
# View current config structure
ls -la config/

# View defaults
ls -la config/defaults/

# Test model-specific override
mv config/<model>/angel_helper.toml /tmp/
cargo run  # Should fall back to defaults
mv /tmp/angel_helper.toml config/<model>/
```

### Troubleshooting Quick Reference

**Connection Pool Exhaustion:**
- Symptom: Hangs after ~5 HTTP API calls, no timeout errors.
- Root Cause: Creating `reqwest::Client::new()` in hot paths (each intel unit call).
- Solution: Pass shared client through `IntelContext`.

**Shell Command Timeouts:**
- Symptom: 30-minute timeouts for simple tasks.
- Causes: Model hangs in retry loops, shell syntax issues, 30-minute timeout too long.
- Solution: Reduce to 5-minute timeout, fix shell command syntax.

**Terminology Mismatch:**
- Symptom: All requests routed to CHAT with entropy=0.00.
- Root Cause: Model tuned on old terminology (CHAT, SHELL), new terms not recognized.
- Solution: Revert to original terminology or perform full re-tuning.

**Pattern-Matching Routing:**
- Symptom: Over-orchestration, keyword-based decisions.
- Root Cause: Hardcoded word patterns in routing logic.
- Solution: Use confidence-based fallback (entropy > 0.8 → CHAT).

---

## 🚀 Future Enhancements

1. **Adaptive Re-tuning**: Re-run JSON tuning if parse errors exceed threshold during normal operation
2. **Per-Difficulty Temperatures**: Use different temperatures for different task complexities
3. **Model-Specific Profiles**: Store and reuse optimal temperatures per model
4. **Continuous Learning**: Update temperature based on runtime JSON success rate
5. **Hierarchical Planning**: Transition from simple step sequences to multi-turn goal persistence
6. **Gated Connectivity**: Maintain `FETCH` in DISABLED state with explicit warnings until sandboxing audit
