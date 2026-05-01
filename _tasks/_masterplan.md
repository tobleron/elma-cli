# Master Plan

**Last updated:** 2026-05-01

This is the execution index for all current pending tasks. Use it to choose work in dependency order, not as a replacement for each task file. Each task file remains the source of implementation detail, verification commands, and done criteria.

## End Goal

Elma produces **correct, grounded answers on 3B-class models**, taking as many intel unit calls and as much wall-clock time as needed. Speed is never the priority — accuracy is.

See [`_objectives.md`](_objectives.md) for the full statement.

## Operating Rules

- Move a task from `_tasks/pending/` to `_tasks/active/` before implementation.
- Complete the task's verification section before marking done.
- Do not modify `src/prompt_core.rs` or `TOOL_CALLING_SYSTEM_PROMPT` unless a task explicitly has user approval.
- Prefer completing each task in priority order. Within a tier, tasks can run in parallel only when their dependencies do not overlap.
- When a model fails on a task, decompose it — never escalate to a larger model or return to JSON.

## Priority Order (Gain per Unit of Effort)

### Tier 1: Direct DSL Repair & Simplification (Highest ROI)

These tasks give the biggest immediate improvement for 3B DSL reliability with the least engineering effort.

| Order | Task | Title | Effort | Gain | Depends On |
|-------|------|-------|--------|------|------------|
| ~~1~~ | ~~418~~ | ~~Concrete repair hints for DSL parse failures~~ | ~~small-med~~ | ~~critical~~ | DONE |
| 2 | [417](pending/417_Simplify_Turn_Summarizer_To_Single_Decision_DSL.md) | Simplify turn summarizer to two-field DSL | small-med | high — eliminates prose/hallucination in summaries | 411 |
| 3 | [412](pending/412_Deterministic_Fallback_On_Repeated_Classifier_DSL_Parse_Failure.md) | Deterministic fallback on repeated classifier DSL parse failure | small | medium-high — safety net for classifier parse loops | none |

### Tier 2: Action DSL Decomposition (Medium ROI)

Splits the "decide action AND format DSL" combined cognitive load into separate narrow units. Higher effort but addresses the root cause of action DSL failures.

| Order | Task | Title | Effort | Gain | Depends On |
|-------|------|-------|--------|------|------------|
| 4 | [416](pending/416_Action_Type_Selector_Intel_Unit.md) | Action type selector intel unit | medium | high — single-field action decision | none |
| 5 | [414](pending/414_Split_Evidence_Needs_Assessor_Into_Single_Field_Units.md) | Split evidence needs assessor into single-field units | medium | medium-high — one decision per unit | none |
| 6 | [415](pending/415_Action_DSL_Format_Specialist_Intel_Unit.md) | Action DSL format specialist intel unit | medium | high — GBNF-constrained formatting | 414, 416 |

### Tier 3: Profile & Boundary Hardening

Broader improvements that touch existing infrastructure.

| Order | Task | Title | Effort | Gain | Depends On |
|-------|------|-------|--------|------|------------|
| 7 | [387](pending/387_Intel_DSL_Profile_Repair_And_Live_Smoke_Gate.md) | Intel DSL profile repair and live smoke gate | medium | medium — tightens existing prompts | 380, 381, 384, 385 |
| 8 | [392](pending/392_Provider_Markup_To_Action_DSL_Boundary.md) | Provider markup to action DSL boundary | small-med | medium — partially done, remaining is transcript + smoke | 378, 391 |
| 9 | [399](pending/399_Prompt_Core_DSL_Rewrite_Review_Gate.md) | Prompt core DSL rewrite review gate | small | high — but requires explicit user approval | none |

### Tier 4: Execution & Visibility Improvements

Important but not directly fixing DSL failures — these improve the quality of answers and system observability after DSL reliability is achieved.

| Order | Task | Title | Effort | Gain | Depends On |
|-------|------|-------|--------|------|------------|
| 10 | [398](pending/398_Evidence_Grounded_Finalization_Reset.md) | Evidence grounded finalization reset | medium | medium — clean finalization after repair/tool loops | 391, 395 |
| 11 | [394](pending/394_Objective_Goal_Task_Action_Pyramid.md) | Objective goal task action pyramid | medium-large | medium — decomposition for complex requests | 380, 393 |
| 12 | [395](pending/395_Transcript_Native_Hidden_Process_Visibility.md) | Transcript native hidden process visibility | medium | low-medium — observability, not model output | 385, 391 |
| 13 | [396](pending/396_Llama_3_2_Fast_DSL_Profile_Certification.md) | Llama 3.2 fast DSL profile certification | medium | low — capstone: certify, not fix | 385, 387, 392, 393 |

## Completed in This Session (2026-05-01)

Tasks that were already implemented in code and moved from pending to completed:

| Task | What | Where |
|------|------|-------|
| 405 | Chat short-circuit gate (INSTRUCT→CHAT prevented by strict thresholds) | `routing_config.rs` |
| 406 | Empty base_url crash (saved_base_url with fallback) | `llm_config.rs` |
| 407 | Hard max iterations cap (40) | `stop_policy.rs` |
| 408 | Fuzzy stagnation detection (action verb tracking) | `stop_policy.rs` |
| 409 | Force stop on repeated DSL failures (check_should_stop) | `stop_policy.rs` |
| 410 | Dedup current message from conversation history | `defaults_evidence_core.rs` |
| 411 | Turn summarizer tool usage from execution trace | `app_chat_loop.rs` |
| 413 | Skip evidence assessment for all CHAT routes | `app_chat_loop.rs` |
| 418 | Concrete repair hints for DSL parse failures | `dsl/repair_templates.rs`, `dsl/render.rs`, `ui/ui_chat.rs`, `routing_infer.rs`, `decomposition.rs` |

## Deferred (Not Current Priority)

These tasks remain in the backlog but are not required for the current 3B stability objective:
- Task 337: File context tracker and stale-read gate
- Task 361: Workspace policy files for ignored/protected paths
- Task 302: Semantic continuity tracking (indirectly helps by catching misalignment, but lower ROI than Tier 1-2)
- Tasks 385-386: Refactored into the above tiers
- Advanced feature work (Waves 4-9 from prior plan preserved for reference below)

---

## Historical Reference: Prior Wave Plans

The sections below are preserved for reference. They represent the full architecture roadmap including completed DSL migration work and deferred feature work. Current focus is Tier 1 above.

### Wave 0: Compact DSL Model-Output Migration (Completed)

- Tasks 376-384: DSL parser, executor, intel unit migration, grammar, certification, JSON removal. All completed.

### Wave 1-9: (Preserved from prior plan — see git history for full details)

Advanced feature work deferred until after 3B model stability is achieved on the basic workflow.
