# Task 096: Chat Classification Pipeline Rebuild

## Continuation Checklist
- [ ] Re-read this task and all linked source/task references before editing.
- [ ] Confirm the task is still valid against current `_tasks/TASKS.md`, `AGENTS.md`, and active master plans.
- [ ] Move or keep this task in `_tasks/active/` before implementation work begins.
- [ ] Inspect the current code/config/docs touched by this task and note any drift from the written plan.
- [ ] Implement the smallest coherent change set that satisfies the next unchecked item.
- [ ] Add or update focused tests, probes, fixtures, or snapshots for the changed behavior.
- [ ] Run `cargo fmt --check` and fix formatting issues.
- [ ] Run `cargo build` and resolve all build errors or warnings introduced by this task.
- [ ] Run targeted `cargo test` commands and any task-specific probes listed below.
- [ ] Run real CLI or pseudo-terminal verification for any user-facing behavior.
- [ ] Record completed work, verification output, and remaining gaps in this task before stopping.
- [ ] Ask for sign-off before moving this task to `_tasks/completed/`.

## Priority
**P0 - ARCHITECTURAL IMPROVEMENT**

## Status
**PENDING** — Ready for implementation

## Objective
Rebuild Elma's classification and context-narrative pipeline to be more reliable, lower-latency, and better grounded for small local models. Reduce pre-orchestrator LLM calls from 8 to 3 (intent + advisor + maestro).

---

## Phase A: Cleanup & Disable Old Architecture (Steps A1-A3)
**Goal:** Remove dead weight so new code doesn't fight old code

**Note:** Steps A1-A3 are deeply entangled with the entire program building pipeline.
The formula selector, classifier, and complexity assessor are referenced throughout
`app_chat_loop.rs`, `orchestration_core.rs`, `orchestration_planning.rs`, and
`evaluation_response.rs`. They cannot be removed in isolation — they must be removed
together with Phase E (orchestrator loop rebuild) which replaces the old program
generation path entirely.

### Step A1: Disable Formula Selector & Classifier Pipeline *(deferred to Phase E)*
- [ ] Remove `FormulaSelectorUnit` from `src/intel_units/intel_units_core.rs`
- [ ] Remove `trait_select_formula` calls from `orchestration_planning.rs`
- [ ] Remove `FormulaSelection` return from `derive_planning_prior_with_ladder`
- [ ] Remove `formula` parameter from `build_program` chain
- [ ] Remove classifier pipeline from `app_chat_loop.rs` (`infer_route_prior` call)
- [ ] Remove `RouteDecision` dependency where no longer needed
- [ ] Remove `route` field from ladder assessment

### Step A2: Remove EvidenceNeeds and ActionNeeds Intel Units *(deferred to Phase E)*
- [ ] Remove `EvidenceNeedsUnit` from `intel_units_core.rs`
- [ ] Remove `ActionNeedsUnit` from `intel_units_core.rs`
- [ ] Remove their calls from `assess_execution_level` in `execution_ladder/mod.rs`
- [ ] Remove `needs_evidence`, `needs_tools`, `needs_decision`, `needs_plan` from complexity assessor schema
- [ ] Update complexity assessor `post_flight` validation

### Step A3: Remove Complexity Assessor as Pre-Flight Unit *(deferred to Phase E)*
- [ ] Remove `ComplexityAssessmentUnit` from pre-flight chain in `assess_execution_level`
- [ ] Keep the struct for future post-program strategic gate use
- [ ] Remove `complexity`, `risk` params from program builder where they were pre-flight inputs
- [ ] Update `orchestration_core.rs` to not pass complexity to orchestrator

---

## Phase B: Expert Advisor Output Simplification (Steps B1-B2)

### Step B1: Collapse ExpertAdvisorAdvice to Single Field
- [ ] `ExpertAdvisorAdvice` struct → only `expert_advice: String`
- [ ] Update `request_response_advice_via_unit` return type
- [ ] Update `present_result_via_unit` to accept `&str`
- [ ] Update `maybe_revise_presented_result` parameter
- [ ] Update `generate_final_answer_once` to pass `String`
- [ ] Update narrative builder to render plain text instead of JSON

### Step B2: Update Expert Advisor System Prompt
- [ ] `prompt_constants.rs` → JSON output: `{"expert_advice": "..."}`
- [ ] `config/defaults/expert_advisor.toml` → update system_prompt
- [ ] Update all per-model `expert_advisor.toml` configs via `sync_and_upgrade_profiles`

---

## Phase C: New Step Types & Respond (Steps C1-C2)

### Step C1: Add Respond and Investigate Step Types
- [ ] Add `Respond` variant to `Step` enum in `types_core.rs`
- [ ] Add `Investigate` variant to `Step` enum
- [ ] Add `Delete` variant (for completeness)
- [ ] Update `handle_program_step` dispatcher for `Respond` and `Investigate`
- [ ] `Respond` handler: no tool call, just stores as final_reply
- [ ] `Investigate` handler: reads context, forms hypotheses, explores — implemented as read+search+reasoning loop

### Step C2: Deterministic Risk Computation
- [ ] Create `compute_program_risk(program: &Program) -> RiskLevel` function
- [ ] Risk mapping: Read/Search/Select/Decide/Plan/MasterPlan/Respond/Investigate → Low, Write/Shell → Medium, Edit/Delete → High
- [ ] Replace all LLM-based risk fields with computed risk
- [ ] Remove `risk` from complexity assessor schema entirely

---

## Phase D: Maestro Intel Unit (Steps D1-D3)

### Step D1: Create the_maestro Intel Unit
- [ ] New module: `src/intel_units/intel_units_maestro.rs`
- [ ] `MaestroUnit` struct implementing `IntelUnit` trait
- [ ] System prompt: "You are Elma's maestro. Generate a numbered list of high-level instructions to achieve the user's objective..."
- [ ] Output schema: `{"steps": [{"num": int, "instruction": string}]}`
- [ ] Context: user message, intent, expert advice, workspace facts, workspace brief, conversation, available capabilities (plain English)
- [ ] Register in `app_bootstrap_profiles.rs` → `maestro_cfg` profile
- [ ] Add `maestro_cfg` to `LoadedProfiles` struct in `app.rs`
- [ ] Add `maestro` to `config_healthcheck.rs` validation

### Step D2: Maestro System Prompt & Profile Config
- [ ] Create `config/defaults/maestro.toml` with canonical prompt
- [ ] Prompt includes: user message, intent, expert advice, workspace facts, workspace brief, capabilities in plain English
- [ ] Capabilities string: "Elma can execute commands, read files, search text, edit files, select from lists, investigate codebases, and respond to users"

### Step D3: Maestro Error Recovery
- [ ] If JSON parsing fails → retry with instruction to "generate fewer instructions"
- [ ] Fallback: single instruction `{"num": 1, "instruction": "Address the user's request"}`

---

## Phase E: Orchestrator Loop (Steps E1-E3)

### Step E1: Single-Instruction Orchestration
- [ ] New function: `orchestrate_instruction(instruction: &str, previous_steps: &[Step])`
- [ ] New user content builder for single-instruction transformation
- [ ] Each call produces 1-3 structured steps for ONE maestro instruction
- [ ] Orchestrator system prompt updated: "Transform this English instruction into 1-3 structured JSON steps..."
- [ ] Add `maestro_instruction` to `config/defaults/orchestrator.toml` prompt

### Step E2: Accumulation Loop with Dependency Wiring
- [ ] Loop: for each maestro instruction → call orchestrator → accumulate steps
- [ ] Wire `depends_on`: instruction N's first step depends on instruction N-1's last step
- [ ] Track step counter (`s1`, `s2`, `s3`...) across iterations
- [ ] Pass accumulated steps as context to each iteration

### Step E3: Summarize→Respond Auto-Append
- [ ] After all maestro instructions processed: check if last step is NOT a reply
- [ ] If not reply → append Summarize step + Respond step
- [ ] If 1 step total and it's Respond → no append needed

---

## Phase F: Execution Loop Integration (Steps F1-F3)

### Step F1: Wire Maestro→Orchestrator→Execution in Chat Loop
- [ ] Replace old `build_program` call in `app_chat_orchestrator.rs`
- [ ] New flow: intent_helper → expert_advisor → the_maestro → orchestrator loop → program → execute
- [ ] Remove `route_decision`, `complexity`, `scope`, `formula` params from program builder

### Step F2: Remove Old Single-Shot Program Generation
- [ ] Remove `request_program_or_repair` function
- [ ] Remove `build_orchestrator_user_content` (old version)
- [ ] Remove recovery path that called old orchestrator
- [ ] Clean up `orchestration_retry.rs` to use new single-instruction recovery

### Step F3: Clean Up Remaining Old Code
- [ ] Remove `ClassificationFeatures` struct
- [ ] Remove `infer_route_prior` function
- [ ] Remove `routing_infer.rs` entirely (or keep tests only)
- [ ] Remove `routing_calc.rs` if unused
- [ ] Remove `routing_parse.rs` if unused
- [ ] Update `config_healthcheck.rs` to remove classifier/profile validation

---

## Phase G: Stress Testing & Verification (Steps G1-G3)

### Step G1: Update Existing Stress Tests
- [ ] Update `run_stress_cli.sh` to validate new pipeline (intent + advice + maestro + steps)
- [ ] Add semantic validators for:
  - Greeting prompts → should produce 1-step Respond
  - File lookup prompts → should produce 2+ step Inspect → Respond
  - Multi-file refactor → should produce 4+ step Read → Edit → Verify → Respond
  - Master planning → should produce multi-chunk output

### Step G2: Create New Maestro-Specific Stress Tests
- [ ] `_stress_testing/T001_Maestro_Greeting.md` — "hi" → maestro produces 1 instruction
- [ ] `_stress_testing/T002_Maestro_FileLookup.md` — "list files in X" → 2-3 instructions
- [ ] `_stress_testing/T003_Maestro_MultiStep.md` — "find bug, fix it, verify" → 4+ instructions
- [ ] `_stress_testing/T004_Maestro_Investigation.md` — "why does X fail" → investigate instruction
- [ ] `_stress_testing/T005_Orchestrator_Transform.md` — verify orchestrator converts maestro instruction to correct step types
- [ ] `_stress_testing/T006_Dependency_Wiring.md` — verify depends_on chains correctly across instructions
- [ ] `_stress_testing/T007_Closing_Rule.md` — verify Summarize→Respond auto-append

### Step G3: Full Verification Gate
- [ ] `cargo build` zero warnings
- [ ] `cargo test` all green
- [ ] `./run_intention_scenarios.sh` all green
- [ ] `./reliability_probe.sh` all green
- [ ] `./run_stress_cli.sh` S000A-S000I all pass
- [ ] New T001-T007 all pass
- [ ] Real CLI: `hi` → Respond, `pwd` → Read+Respond, `cwd` → Read+Respond, `shell scripts?` → Search+Respond

---

## Final Pipeline (After All Steps)

```
User message → intent_helper → "The user is asking X"
             → expert_advisor → "The best way is to..."
             → the_maestro    → [{"num":1,"instruction":"..."}, {"num":2,"instruction":"..."}]
             → orchestrator loop (per instruction):
                 instruction 1 → [s1, s2, s3]
                 instruction 2 → [s4]
             → auto-append Summarize→Respond if needed
             → execute all steps
             → risk = max(step types)  [deterministic]
             → result_presenter → final answer
```

## Context Narrative Format

```
User message:
are there any shell scripts on root folder?
[intent: The user is asking if there are shell scripts in the root folder.]
[Elma's thoughts: The best way is to inspect the root directory to list shell script files.]

Workspace facts:
cwd: /Users/r2/elma-cli
user: r2
shell: /bin/zsh
os: macOS
git_branch: main
git_status: 142 file(s) changed

Workspace brief:
AGENTS.md
Cargo.lock
Cargo.toml
_dev-system/
_dev-tasks/
_scripts/
...

Conversation so far (most recent last):
user: hi
assistant: hey
user: are there any shell scripts on root folder?
```

---

## Session Issues Fixed (Post-Phase G)

**Issue 1: JSON repair always runs, even when parse succeeds**
- **Fix:** `orchestration_helpers/mod.rs` — `request_program_or_repair` now tries `parse_json_loose` first and only calls repair if it actually fails
- **Impact:** Eliminates wasted LLM call and wrong recovery path when valid JSON is declared broken

**Issue 2: "Retry" trace is misleading**
- **Fix:** `orchestration_retry.rs` — Renamed "Retry N/M" → "Strategy attempt N/M" and trace key `orchestration_retry_attempt` → `orchestration_strategy_attempt`
- **Impact:** Trace now accurately describes what's happening — this is a strategy chain attempt, not a retry of a previous attempt

**Issue 3: Result presenter hallucinates fake output when no evidence exists**
- **Fix:** `prompt_constants.rs` — Added HONESTY RULE to result presenter: explicitly forbids inventing file listings, command output, or fake content when no evidence-gathering steps were executed
- **Impact:** When no shell/read/search steps ran, presenter states honestly what happened instead of fabricating output

**Issue 4 (Deferred): Orchestrator ignores user's actual command**
- `ls -ltr` → generated `ls -1 src/` instead
- Root cause: Maestro instruction overrides user message in orchestrator prompt
- Requires discussion before fix

## JSON Repair Fix (Corrected Implementation)

**Problem:** `parse_json_loose` in `routing_parse.rs` always runs `validate_no_repetition_loop` FIRST, which can falsely flag valid 3B model output as a repetition loop. Then it falls through to the LLM-based JSON repair specialist, which wastes an LLM call and often declares valid JSON broken.

**Fix:** Added `serde_json::from_str` as a fast path at the TOP of `parse_json_loose`. If serde_json natively parses the JSON, we trust it completely and skip ALL custom validation (repetition check, custom parser, repair). The standard library is deterministic — no LLM repair needed.

**File:** `src/routing_parse.rs` — `parse_json_loose` function
