# 🎯 ELMA-CLI RE-PRIORITIZED ROADMAP

## Your 4 Foundational Pillars (P0 - BLOCKING)

**These 4 objectives MUST be completed before any other feature work.**

| Pillar | Objective | Status |
|--------|-----------|--------|
| **P0-1** | JSON Model Output: 100% Reliability (Grammar + Parsing + Repair + Intel Unit) | 🔴 NOT STARTED |
| **P0-2** | Context Narrative: Each workflow unit receives appropriate narrative (not JSON) | 🟡 PARTIAL (Task 047 started) |
| **P0-3** | Workflow Sequence: Optimal configuration (what comes first?) | 🔴 NOT STARTED |
| **P0-4** | Then: All other reliability tasks (error handling, persistence, etc.) | ⏸️ BLOCKED on P0-1,2,3 |

---

## 📋 COMPLETE RE-PRIORITIZED TASK LIST

### **PILLAR 1: JSON RELIABILITY (100% Guaranteed Output)**

#### **Task 001: GBNF Grammar Enforcement for All Intel Units** 🔴 NEW
**Priority:** P0-1.1 (CRITICAL - FIRST)
**Status:** NOT STARTED

**Objective:** Enforce 100% valid JSON output from all intel units using GBNF grammars.

**Technical Tasks:**
- [ ] Audit all 78 default profiles for JSON output requirements
- [ ] Create GBNF grammar files for each output type:
  - `grammar_choice_1of2.json.gbnf` (router, speech_act, mode_router)
  - `grammar_choice_with_reason.json.gbnf` (complexity, evidence_needs, action_needs)
  - `grammar_structured_decision.json.gbnf` (workflow_planner, formula_selector)
  - `grammar_step_list.json.gbnf` (program generation)
- [ ] Update `Profile` struct to include optional `grammar` field
- [ ] Modify `chat_once()` to send grammar with request
- [ ] Test grammar enforcement for all intel units

**Files to Create:**
- `config/grammars/` directory
- `src/json_grammar.rs` - Grammar loading and injection

**Acceptance Criteria:**
- ✅ All intel unit responses are 100% valid JSON (zero parse failures)
- ✅ Grammar files documented and versioned
- ✅ No performance degradation (>95% of current speed)

**Dependencies:** None (foundational)

---

#### **Task 002: Robust JSON Parser with Auto-Repair** 🔴 NEW
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
- [ ] Implement multi-pass parsing:
  1. Try standard `serde_json::from_str()`
  2. Try `jsonrepair-rs::repair()`
  3. Try manual repair (remove comments, fix quotes)
  4. Extract fields with regex fallback
- [ ] Log all repairs for analysis
- [ ] Add unit tests for each repair strategy

**Files to Modify:**
- `src/json_parser.rs` - Complete rewrite
- `src/json_error_handler.rs` - Integrate with circuit breaker

**Acceptance Criteria:**
- ✅ 99%+ parse success rate (currently ~85-90%)
- ✅ All repairs logged with before/after
- ✅ Fallback to regex extraction if all else fails
- ✅ Circuit breaker only opens after 10 consecutive failures (not 5)

**Dependencies:** Task 001 (GBNF reduces repair load)

---

#### **Task 003: JSON Repair Intel Unit (Advanced)** 🔴 NEW
**Priority:** P0-1.3 (CRITICAL - ADVANCED FEATURE)
**Status:** NOT STARTED

**Objective:** Dedicated intel unit that repairs malformed JSON using model intelligence.

**Technical Tasks:**
- [ ] Create `src/intel_units/json_repair_unit.rs`
- [ ] Design prompt for JSON repair:
  ```
  You are Elma's JSON Repair Unit.
  
  Given malformed JSON output, fix it while preserving the original meaning.
  
  Input (malformed):
  {raw_output}
  
  Output format (must be valid JSON):
  {expected_schema}
  
  Return ONLY the corrected JSON. No explanations.
  ```
- [ ] Create `config/defaults/json_repair_intel.toml` profile
- [ ] Integrate into parsing pipeline:
  1. Standard parse → fail
  2. Auto-repair → fail
  3. **JSON Repair Intel Unit** → should succeed
  4. Last resort: fallback values
- [ ] Add retry limit (max 2 repair attempts)
- [ ] Log all repairs for prompt tuning

**Files to Create:**
- `src/intel_units/json_repair_unit.rs`
- `config/defaults/json_repair_intel.toml`

**Acceptance Criteria:**
- ✅ Repairs JSON that auto-repair cannot fix
- ✅ Preserves semantic meaning (not just syntax)
- ✅ Adds <500ms latency
- ✅ 99.5%+ total parse success rate

**Dependencies:** Task 002 (auto-repair first, intel unit second)

---

#### **Task 004: Schema Validation for All Intel Outputs** 🔴 NEW
**Priority:** P0-1.4 (CRITICAL)
**Status:** NOT STARTED

**Objective:** Validate all intel unit outputs against schemas before use.

**Technical Tasks:**
- [ ] Define schemas for all intel output types:
  ```rust
  pub struct IntelSchema {
      pub required_fields: Vec<&'static str>,
      pub field_types: HashMap<&'static str, FieldType>,
      pub validators: Vec<FieldValidator>,
  }
  ```
- [ ] Implement validation in `json_error_handler.rs`:
  - Check required fields present
  - Check field types match
  - Run custom validators (e.g., entropy 0.0-1.0)
- [ ] Add schema to each intel unit profile
- [ ] Reject invalid outputs and trigger repair
- [ ] Log validation failures for tuning

**Files to Modify:**
- `src/json_error_handler.rs` - Add schema validation
- `src/types_api.rs` - Add schema definitions
- All intel unit configs - Add schema field

**Acceptance Criteria:**
- ✅ All intel outputs validated before use
- ✅ Invalid outputs trigger repair, not crash
- ✅ Validation errors logged with field details
- ✅ No semantic drift (repaired output matches intent)

**Dependencies:** Task 003 (repair pipeline must exist first)

---

### **PILLAR 2: CONTEXT NARRATIVE (Right Context for Each Unit)**

#### **Task 005: Narrative Format Specification** 🔴 NEW
**Priority:** P0-2.1 (CRITICAL)
**Status:** NOT STARTED

**Objective:** Define narrative format standards for all workflow units.

**Technical Tasks:**
- [ ] Audit all 78 default profiles for current input format
- [ ] Classify units by narrative needs:
  - **Classification units** (router, speech_act, mode_router): User message + workspace + route priors
  - **Planning units** (workflow_planner, complexity, scope): User message + route + workspace + conversation
  - **Execution units** (tooler, decider, selector): Step purpose + objective + prior results
  - **Verification units** (critic, sufficiency, reviewers): Objective + steps + results + attempt
  - **Repair units** (program_repair, command_repair): Failed program + error + user intent
- [ ] Design narrative template for each class
- [ ] Document narrative principles:
  - Plain text, not JSON blobs
  - No entropy/distribution fields (except where needed)
  - Consistent section headers
  - Ultra-concise (minimize tokens)
- [ ] Create `docs/NARRATIVE_STANDARD.md`

**Files to Create:**
- `docs/NARRATIVE_STANDARD.md`
- `src/intel_narrative.rs` - Expand with all templates

**Acceptance Criteria:**
- ✅ All 78 profiles classified by narrative type
- ✅ Narrative templates documented
- ✅ Token count reduced by 30% vs. JSON format
- ✅ Model accuracy maintained or improved

**Dependencies:** None (can parallelize with Task 001-004)

---

#### **Task 006: Extend Intel Narrative Module to All Units** 🟡 EXISTING (Task 047 → 006)
**Priority:** P0-2.2 (CRITICAL)
**Status:** PARTIAL (Phase 1 done for critic)

**Objective:** Implement narrative builders for ALL intel units (not just critic).

**Technical Tasks:**
**Phase 1: Classification Narratives**
- [ ] `build_complexity_narrative()` - Already exists, verify format
- [ ] `build_evidence_needs_narrative()` - Already exists, verify format
- [ ] `build_action_needs_narrative()` - Already exists, verify format
- [ ] `build_route_narrative()` - NEW for routing units

**Phase 2: Planning Narratives**
- [ ] `build_workflow_planner_narrative()` - NEW
- [ ] `build_scope_builder_narrative()` - NEW
- [ ] `build_formula_selector_narrative()` - NEW

**Phase 3: Execution Narratives**
- [ ] `build_tooler_narrative()` - NEW
- [ ] `build_decider_narrative()` - NEW
- [ ] `build_selector_narrative()` - NEW
- [ ] `build_planner_narrative()` - NEW

**Phase 4: Verification Narratives**
- [ ] `build_sufficiency_narrative()` - TODO
- [ ] `build_reviewer_narrative()` - TODO (logical, efficiency, risk)
- [ ] `build_outcome_verifier_narrative()` - NEW

**Phase 5: Repair Narratives**
- [ ] `build_program_repair_narrative()` - NEW
- [ ] `build_command_repair_narrative()` - NEW

**Files to Modify:**
- `src/intel_narrative.rs` - Add all narrative builders
- All intel unit callers - Use narrative instead of JSON

**Acceptance Criteria:**
- ✅ All intel units use narrative format
- ✅ No JSON entropy/distribution in intel input (except where needed)
- ✅ Tests passing for all updated units
- ✅ Stress tests show improved accuracy

**Dependencies:** Task 005 (narrative spec first)

---

#### **Task 007: Context Boundary Enforcement** 🔴 NEW
**Priority:** P0-2.3 (CRITICAL)
**Status:** NOT STARTED

**Objective:** Ensure each unit receives ONLY the context it needs (no over-sharing).

**Technical Tasks:**
- [ ] Audit current context sharing:
  - Which units receive full conversation?
  - Which units receive workspace facts?
  - Which units receive route decisions?
- [ ] Define context boundaries per unit class:
  ```rust
  pub enum ContextBoundary {
      Minimal,      // User message only
      Classification, // + workspace + route
      Planning,     // + conversation + complexity
      Execution,    // + prior results + artifacts
      Verification, // + full program + all results
  }
  ```
- [ ] Implement context builder:
  ```rust
  pub fn build_context(boundary: ContextBoundary, ctx: &FullContext) -> IntelContext {
      match boundary {
          Minimal => IntelContext { user_message: ctx.user_message, .. },
          Classification => IntelContext { ... },
          // ...
      }
  }
  ```
- [ ] Update all intel unit calls to use appropriate boundary
- [ ] Log context size for each unit (token counting)

**Files to Create:**
- `src/context_boundary.rs`

**Files to Modify:**
- `src/intel_trait.rs` - Add boundary field
- All intel unit callers - Specify boundary

**Acceptance Criteria:**
- ✅ Each unit receives minimal sufficient context
- ✅ Token usage reduced by 20%+ vs. current
- ✅ No accuracy regression
- ✅ Context boundaries documented

**Dependencies:** Task 006 (narrative format first)

---

### **PILLAR 3: WORKFLOW SEQUENCE OPTIMIZATION**

#### **Task 008: Workflow Sequence Analysis** 🔴 NEW
**Priority:** P0-3.1 (CRITICAL)
**Status:** NOT STARTED

**Objective:** Analyze and document optimal workflow sequence.

**Technical Tasks:**
- [ ] Document CURRENT sequence:
  ```
  1. User input
  2. Intent annotation (intent_helper)
  3. Classification (speech_act × workflow × mode → route)
  4. Planning priors (complexity, evidence_needs, action_needs)
  5. Workflow planner
  6. Formula selector
  7. Scope builder
  8. Program building
  9. Execution
  10. Verification (sufficiency → critics → reviewers)
  11. Refinement (if needed)
  ```
- [ ] Identify sequence problems:
  - Is intent annotation before or after classification? (Currently: before)
  - Should complexity assessment come before route decision? (Currently: after)
  - Should ladder assessment come before or after formula selection?
  - Should verification happen per-step or post-program? (Currently: both)
- [ ] Analyze dependencies:
  - What does each unit NEED as input?
  - What does each unit PRODUCE as output?
  - Can any units run in parallel?
- [ ] Propose OPTIMAL sequence:
  ```
  PROPOSED:
  1. User input
  2. Intent annotation (lightweight)
  3. Route classification (speech_act × workflow × mode)
  4. Execution ladder (minimum sufficient level)
  5. Complexity assessment (depends on ladder)
  6. Evidence/action needs (depends on complexity)
  7. Formula selection (depends on needs + ladder)
  8. Scope builder (depends on formula)
  9. Workflow planner (ONLY if ladder >= Task)
  10. Program building
  11. Execution
  12. Verification (per-step + post-program)
  13. Refinement (if needed)
  ```
- [ ] Create `docs/WORKFLOW_SEQUENCE.md` with dependency graph

**Files to Create:**
- `docs/WORKFLOW_SEQUENCE.md`

**Acceptance Criteria:**
- ✅ Current sequence fully documented
- ✅ Dependency graph complete
- ✅ Optimal sequence proposed with justification
- ✅ Performance model (which sequence minimizes tokens/latency)

**Dependencies:** None (analysis task)

---

#### **Task 009: Workflow Sequence Reordering** 🔴 NEW
**Priority:** P0-3.2 (CRITICAL)
**Status:** NOT STARTED

**Objective:** Implement optimal workflow sequence from Task 008.

**Technical Tasks:**
- [ ] Refactor `app_chat_core.rs::run_chat_loop()`:
  - Move ladder assessment before complexity
  - Make workflow planner conditional (only if ladder >= Task)
  - Parallelize independent units (evidence_needs + action_needs)
- [ ] Update `derive_planning_prior_with_ladder()`:
  - Accept ladder as input (not compute internally)
  - Return only what's needed for next step
- [ ] Add sequence validation:
  ```rust
  pub fn validate_sequence(ctx: &WorkflowContext) -> Result<()> {
      // Check ladder set before complexity
      // Check complexity set before formula
      // Check formula set before scope
      // ...
  }
  ```
- [ ] Add tracing for sequence timing:
  - Log duration of each step
  - Identify bottlenecks
- [ ] Test with scenarios to ensure behavioral equivalence

**Files to Modify:**
- `src/app_chat_core.rs` - Main sequence
- `src/execution_ladder.rs` - Move earlier in sequence
- `src/intel_units.rs` - Update dependencies

**Acceptance Criteria:**
- ✅ Optimal sequence implemented
- ✅ All scenario tests passing
- ✅ Latency reduced by 15%+ (fewer sequential dependencies)
- ✅ Token usage reduced by 10%+ (conditional workflow planner)

**Dependencies:** Task 008 (analysis first)

---

#### **Task 010: Conditional Workflow Planning** 🔴 NEW
**Priority:** P0-3.3 (CRITICAL)
**Status:** NOT STARTED

**Objective:** Only run workflow planner when ladder >= Task (skip for Action level).

**Technical Tasks:**
- [ ] Modify `build_program()` to check ladder level:
  ```rust
  if ladder.level == ExecutionLevel::Action {
      // Direct program building (no workflow planner)
      build_direct_program(user_message, route_decision, workspace)?
  } else {
      // Use workflow planner for Task/Plan/MasterPlan
      build_planned_program(workflow_plan, ladder, formula)?
  }
  ```
- [ ] Create `build_direct_program()` for Action-level tasks:
  - Single operation (Shell, Read, Search, Edit, Reply)
  - No decomposition needed
  - Minimal context required
- [ ] Update formula definitions to include ladder level:
  ```toml
  # strategy_direct.toml
  ladder_level = "Action"
  max_steps = 2
  ```
- [ ] Test with Action-level scenarios:
  - "List files" → Action (direct Shell step)
  - "What is X?" → Action (direct Reply step)
  - "Read file Y" → Action (direct Read step)

**Files to Create:**
- `src/program_building_direct.rs` - Direct program builder

**Files to Modify:**
- `src/app_chat_core.rs` - Conditional workflow planner
- `src/execution_ladder.rs` - Add `allows_direct_execution()` helper

**Acceptance Criteria:**
- ✅ Action-level tasks skip workflow planner
- ✅ Token usage reduced by 30% for Action tasks
- ✅ Latency reduced by 50% for Action tasks
- ✅ No accuracy regression

**Dependencies:** Task 009 (sequence reordering first)

---

#### **Task 011: Parallel Intel Unit Execution** 🔴 NEW
**Priority:** P0-3.4 (OPTIMIZATION)
**Status:** NOT STARTED

**Objective:** Run independent intel units in parallel to reduce latency.

**Technical Tasks:**
- [ ] Identify parallelizable units:
  - `complexity_assessment` + `evidence_needs` + `action_needs` (all depend only on route + user message)
  - `logical_reviewer` + `efficiency_reviewer` + `risk_reviewer` (all depend on step results)
- [ ] Implement parallel execution:
  ```rust
  let (complexity, evidence, action) = tokio::join!(
      assess_complexity(ctx),
      assess_evidence_needs(ctx),
      assess_action_needs(ctx),
  );
  ```
- [ ] Add parallel execution config:
  ```rust
  pub struct ParallelConfig {
      pub max_concurrent: usize,  // Default: 3
      pub timeout_ms: u64,        // Default: 10000
  }
  ```
- [ ] Handle partial failures (if one unit fails, others still complete)
- [ ] Log parallel execution timing

**Files to Create:**
- `src/intel_parallel.rs` - Parallel execution orchestration

**Acceptance Criteria:**
- ✅ Independent units run in parallel
- ✅ Latency reduced by 40%+ for classification phase
- ✅ Partial failures handled gracefully
- ✅ No race conditions or context corruption

**Dependencies:** Task 009 (sequence must be stable first)

---

### **PILLAR 4: RELIABILITY TASKS (Previously Priority, Now After P0-1,2,3)**

#### **Task 012: Comprehensive Error Reporting** 🟡 EXISTING (Task 016 → 012)
**Priority:** P0-4.1 (CRITICAL - but blocked on P0-1,2,3)
**Status:** NOT STARTED

**Objective:** Every session produces actionable error reports or success markers.

**Technical Tasks:**
- [ ] Create `src/error_report.rs` with `SessionError` struct
- [ ] Install panic hook in `main.rs`
- [ ] Write `error.json` on any fatal error
- [ ] Add session status tracking (`session_status.json`)
- [ ] Wrap all entry points with error handling
- [ ] Improve trace logging for errors

**Acceptance Criteria:**
- ✅ No session ends without `error.json` or success marker
- ✅ Panic writes stack trace to log
- ✅ Error types clearly distinguished
- ✅ Error messages actionable

**Dependencies:** P0-1,2,3 must be complete first

---

#### **Task 013: Multi-Turn Goal Persistence** 🟡 EXISTING (Task 014 → 013)
**Priority:** P0-4.2 (CRITICAL - but blocked on P0-1,2,3)
**Status:** NOT STARTED

**Objective:** GoalState persists across session restarts.

**Technical Tasks:**
- [ ] Add `goals_dir` to `SessionPaths`
- [ ] Serialize `GoalState` to `active_goal.json`
- [ ] Load `GoalState` on session start
- [ ] Update goal state after each turn
- [ ] Rollback restores previous goal state

**Acceptance Criteria:**
- ✅ GoalState persists across session restarts
- ✅ User can resume multi-turn workflows
- ✅ Rollback restores previous goal state

**Dependencies:** P0-1,2,3 must be complete first

---

#### **Task 014: Ground Critics in Actual Output** 🟡 EXISTING (Architecture Decision → 014)
**Priority:** P0-4.3 (CRITICAL - but blocked on P0-1,2,3)
**Status:** NOT STARTED

**Objective:** Critics verify grounding in actual step results, not hallucinate failures.

**Technical Tasks:**
- [ ] Fix `critic.toml` prompt (principle-first, grounded)
- [ ] Add `verify_grounding()` function
- [ ] Reject critic verdicts without evidence
- [ ] Log hallucinated failures for tuning

**Acceptance Criteria:**
- ✅ Critics catch real failures only
- ✅ Hallucination rate <5%
- ✅ All critic verdicts grounded in evidence

**Dependencies:** P0-1,2,3 must be complete first (narrative format helps)

---

#### **Task 015: Enable Reflection for ALL Tasks** 🟡 EXISTING (Architecture Decision → 015)
**Priority:** P0-4.4 (CRITICAL - but blocked on P0-1,2,3)
**Status:** NOT STARTED

**Objective:** Reflection runs for all tasks (not just non-DIRECT).

**Technical Tasks:**
- [ ] Remove DIRECT skip condition in `app_chat_core.rs`
- [ ] Add lightweight reflection mode for DIRECT tasks
- [ ] Test reflection accuracy for all complexity levels

**Acceptance Criteria:**
- ✅ Reflection runs for all tasks
- ✅ DIRECT tasks get lightweight reflection (1-2 questions)
- ✅ No latency regression for DIRECT tasks

**Dependencies:** P0-1,2,3 must be complete first

---

#### **Task 016: Workspace Context Optimization** 🟡 EXISTING (Task 044 → 016)
**Priority:** P0-4.5 (HIGH - but blocked on P0-1,2,3)
**Status:** NOT STARTED

**Objective:** Tree view with `ignore` crate, reduce verbosity.

**Technical Tasks:**
- [ ] Implement tree view in `workspace_tree.rs`
- [ ] Add file importance heuristics
- [ ] Limit recent files list
- [ ] Semantic grouping (not flat list)

**Acceptance Criteria:**
- ✅ Token usage reduced by 30%
- ✅ Model accuracy maintained
- ✅ Tree view human-readable

**Dependencies:** P0-1,2,3 must be complete first

---

#### **Task 017: Evidence Compaction** 🟡 EXISTING (Task 020 → 017)
**Priority:** P0-4.6 (HIGH - but blocked on P0-1,2,3)
**Status:** NOT STARTED

**Objective:** Hierarchical evidence compaction to reduce token bloat.

**Technical Tasks:**
- [ ] Create `src/evidence_compaction.rs`
- [ ] Rank evidence by importance
- [ ] Summarize low-importance items
- [ ] Keep high-importance items verbatim

**Acceptance Criteria:**
- ✅ Token usage reduced by 40% for long sessions
- ✅ Evidence integrity maintained
- ✅ Model accuracy improved (less noise)

**Dependencies:** P0-1,2,3 must be complete first

---

#### **Task 018: Rolling Conversation Summary** 🟡 EXISTING (Task 021 → 018)
**Priority:** P0-4.7 (HIGH - but blocked on P0-1,2,3)
**Status:** NOT STARTED

**Objective:** Summarize old messages to prevent context bloat.

**Technical Tasks:**
- [ ] Create `src/session_summary.rs`
- [ ] Summarize every N turns
- [ ] Keep last M messages verbatim
- [ ] Replace older messages with summary

**Acceptance Criteria:**
- ✅ Context stays under token limit
- ✅ Objective preserved across turns
- ✅ No context amnesia

**Dependencies:** P0-1,2,3 must be complete first

---

## 📊 SUMMARY: TASK COUNT BY PILLAR

| Pillar | Tasks | Status |
|--------|-------|--------|
| **P0-1: JSON Reliability** | 4 tasks (001-004) | 🔴 All NEW |
| **P0-2: Context Narrative** | 3 tasks (005-007) | 🔴 2 NEW, 1 renumbered |
| **P0-3: Workflow Sequence** | 4 tasks (008-011) | 🔴 All NEW |
| **P0-4: Reliability Tasks** | 7 tasks (012-018) | 🟡 All renumbered from existing |
| **TOTAL** | **18 tasks** | **P0-1,2,3 BLOCK P0-4** |

---

## 🎯 EXECUTION ORDER (STRICT SEQUENCING)

### **PHASE 1: JSON RELIABILITY (Weeks 1-4)**
1. ✅ Task 001: GBNF Grammar Enforcement
2. ✅ Task 002: Robust JSON Parser with Auto-Repair
3. ✅ Task 003: JSON Repair Intel Unit
4. ✅ Task 004: Schema Validation

### **PHASE 2: CONTEXT NARRATIVE (Weeks 5-7)**
5. ✅ Task 005: Narrative Format Specification
6. ✅ Task 006: Extend Narrative to All Units
7. ✅ Task 007: Context Boundary Enforcement

### **PHASE 3: WORKFLOW SEQUENCE (Weeks 8-10)**
8. ✅ Task 008: Workflow Sequence Analysis
9. ✅ Task 009: Workflow Sequence Reordering
10. ✅ Task 010: Conditional Workflow Planning
11. ✅ Task 011: Parallel Intel Unit Execution

### **PHASE 4: RELIABILITY ENHANCEMENTS (Weeks 11-16)**
12. ✅ Task 012: Comprehensive Error Reporting
13. ✅ Task 013: Multi-Turn Goal Persistence
14. ✅ Task 014: Ground Critics in Actual Output
15. ✅ Task 015: Enable Reflection for ALL Tasks
16. ✅ Task 016: Workspace Context Optimization
17. ✅ Task 017: Evidence Compaction
18. ✅ Task 018: Rolling Conversation Summary

---

## 🚨 CRITICAL CONSTRAINT

**DO NOT START PHASE 4 UNTIL PHASES 1-3 ARE COMPLETE.**

**Rationale:**
- Phase 4 tasks depend on stable JSON pipeline (Phase 1)
- Phase 4 tasks depend on narrative context (Phase 2)
- Phase 4 tasks depend on optimal sequence (Phase 3)
- Adding error handling to broken JSON pipeline = wasted work
- Adding persistence to wrong sequence = persistent wrongness

**Your directive is clear:** Perfect P0-1, P0-2, P0-3 FIRST. Then everything else.

---

## 📝 OLD TASK RENUMBERING MAP

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
| 019+ | POSTPONED | All other tasks | Postponed until P0-1,2,3,4 complete |

---

## ✅ NEXT ACTION

**Start Task 001: GBNF Grammar Enforcement**

This is the absolute foundation. Without 100% valid JSON output, everything else is fragile.

**First steps:**
1. Read all 78 default profiles to identify JSON output types
2. Create `config/grammars/` directory
3. Write first GBNF grammar (choice_1of2)
4. Test with router.toml

**Shall I create Task 001 in `_tasks/active/` and begin implementation?**
