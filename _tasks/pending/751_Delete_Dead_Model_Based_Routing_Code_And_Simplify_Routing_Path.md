# Task 751: Delete Dead Model-Based Routing Code And Simplify Routing Path

## Type

Architecture / Dead Code / Reliability

## Severity

High

## Scope

Routing system, chat loop, orchestration

## Problem

The codebase contains a sophisticated model-based routing system in `src/routing_infer.rs` (330 lines, with entropy/margin thresholds, config-driven thresholds, and test coverage) that appears to be bypassed by the current direct tool-calling path.

In `src/app_chat_loop.rs` lines 745-781, the result of `annotate_and_classify()` (which calls `infer_route_prior`) is **discarded** and replaced with a hardcoded struct:

```rust
let rephrased_objective = line.to_string();
let route_decision = RouteDecision {
    route: "SHELL".to_string(),
    source: "direct_tool_calling".to_string(),
    margin: 0.0,
    entropy: 0.0,
    distribution: vec![("SHELL".to_string(), 1.0)],
    speech_act: ProbabilityDecision { choice: "INSTRUCT".to_string(), ... },
    workflow: ProbabilityDecision { choice: "WORKFLOW".to_string(), ... },
    mode: ProbabilityDecision { choice: "EXECUTE".to_string(), ... },
    evidence_required: false,
};
```

The comment justifies this:

> "Route classification is no longer needed. The model has all tools and decides what to call via the tool loop."

But this rationale needs a cleanup decision:
1. **Potential wasted API calls**: if `annotate_and_classify()` still runs anywhere in the live path, it consumes tokens and latency while its output is thrown away
2. **Complexity assessment depends on routing**: `complexity` and `formula` are derived from `route_decision.evidence_required` and `route_decision.speech_act.choice` — but these now come from hardcoded values, making the complexity/formula selection meaningless
3. **Dead code accumulation**: `routing_infer.rs`, `routing_calc.rs`, `routing_parse.rs`, `routing_config.rs` are all maintained but unused
4. **Semantic continuity violation**: Rule 4 says meaning must survive the pipeline. When routing is always "SHELL" regardless of user intent, the pipeline loses the user's actual intent before execution begins
5. **Delete-first policy violation**: Rule 13 says "If logic has been 'repaired' 3 times, it is architecturally unsound. Rewrite the abstraction rather than patching symptoms." The routing system has been patched with dead-code comments rather than deleted.

## Root Cause

The tool-calling pipeline was introduced as a simpler architecture, but the old routing system was never removed. Developers added comments and hardcoded fallbacks rather than deleting the obsolete code.

## Proposed Solution

### Phase 0 — Prove the live path

Before deleting anything, trace the actual call graph and one real prompt run:

1. Confirm whether `annotate_and_classify()` or `infer_route_prior()` is executed before the tool loop starts.
2. Confirm whether `RouteDecision` and `ProbabilityDecision` are only routing artifacts or still serve as compatibility context for intel units.
3. Record the proof in the task notes or a session artifact.
4. If routing calls are already removed from the live path, skip straight to deleting dead modules and updating docs.
5. If a route type is still structurally required, replace it with a smaller planning context instead of deleting it blindly.

### Phase 1 — Delete dead routing modules

1. Delete `src/routing_infer.rs`
2. Delete `src/routing_calc.rs`
3. Delete `src/routing_parse.rs`
4. Delete `src/routing_config.rs`
5. Delete `src/routing.rs` (façade re-export, unless it has other live content)
6. Remove all module declarations from `main.rs`
7. Remove all `pub(crate) use routing::*` from `main.rs`

### Phase 2 — Delete dead routing types and helpers where proven dead

1. In `src/types_core.rs` or `src/types.rs`: remove `RouteDecision`, `ProbabilityDecision`, `RouterCalibration` only if they are proven to be routing-only
2. In `src/app_chat_loop.rs`: remove `annotate_and_classify()` function entirely
3. Remove `RouteDecision` fields from `IntelContext` if no longer needed
4. Remove routing-related trace calls

### Phase 3 — Simplify complexity and formula selection

If routing is proven bypassed or compatibility-only, simplify the downstream logic:

1. In `app_chat_loop.rs`, replace the complexity/formula construction block (lines 802-858) with a direct call to a simplified `select_formula_for_request()` function
2. This function should take only the user message and workspace brief
3. It should use a lightweight intel unit (or direct model call) to select formula, not derive it from hardcoded routing artifacts
4. The formula selection should still respect complexity assessment after Task 747 fixes it

### Phase 4 — Update docs

1. Remove routing system references from `docs/ARCHITECTURE.md`
2. Update the "End-to-End Flow" diagram to show direct tool-calling without routing
3. Update `docs/SKILL_SYSTEM.md` to remove routing-dependent formula selection paths

## Acceptance Criteria

- [ ] A call-graph proof documents whether routing is live, bypassed, or compatibility-only
- [ ] `src/routing_infer.rs`, `routing_calc.rs`, `routing_parse.rs`, `routing_config.rs` are deleted if proven dead
- [ ] `annotate_and_classify()` is deleted from `app_chat_loop.rs` if no live caller remains
- [ ] No API calls are made for routing before the tool loop starts
- [ ] `RouteDecision` and `ProbabilityDecision` types are deleted only if no longer used; otherwise they are replaced with a smaller planning context in a separate scoped change
- [ ] Complexity and formula selection still work (via simplified path)
- [ ] `cargo build && cargo test` passes
- [ ] `docs/ARCHITECTURE.md` is updated to reflect the simplified flow
- [ ] Session transcript shows no routing-related trace entries unless a documented compatibility shim remains

## Verification Plan

- `find src -name "routing*.rs"` -> no files if routing is proven dead
- `grep -r "annotate_and_classify" src/` -> no matches if no live caller remains
- `grep -r "infer_route_prior" src/` -> no matches if no live caller remains
- `grep -r "RouteDecision" src/` -> no matches, or a documented compatibility shim remains with no routing API call
- Integration test: simple chat request "hello" → no routing API call, tool loop handles it
- Integration test: complex request "refactor the auth module" → no routing API call, tool loop handles it

## Dependencies

- Task 747 (complexity assessment fix) should land first so formula selection has a working complexity input
- `src/skills.rs` (formula selection logic)
- `src/app_chat_loop.rs` (main simplification site)

## Notes

This is a **prove-then-delete** task. The temptation will be to keep a bypassed routing layer for future use or delete compatibility types blindly. Do neither. First prove what is live, then remove dead routing logic while preserving the smaller planning context Elma still needs.

> "If logic has been 'repaired' 3 times, it is architecturally unsound. Rewrite the abstraction rather than patching symptoms."

If model-based routing is ever needed again, it should be rebuilt from first principles using the current intel unit framework, not revived from bypassed legacy code.
