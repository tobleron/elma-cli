# Task 376: Replace Length Heuristic With LLM Route Inference

**Status:** Pending
**Priority:** HIGHEST
**Estimated effort:** 1-2 days
**Dependencies:** None
**References:** AGENTS.md Rule 1, objectives.md, user requirement for tool discovery and rust-native tool preference, session `s_1777658707_425792000`

## Problem

`src/app_chat_loop.rs:846-847` uses a character-count heuristic for routing:

```rust
let likely_conversational =
    line.len() < 30 && !extract_first_path_from_user_text(line).is_some();
```

This sets `route="CHAT"`, `formula="reply_only"`, and `needs_tools=false` for any query shorter than 30 characters. Queries like "what time is it now?" (17 chars) need `bash` access to run `date`, but the heuristic blocks tool access and the 4B model hallucinates an answer.

This violates AGENTS.md Rule 1: "Do Not Turn Elma Into A Keyword Matcher." Character-length classification IS a keyword heuristic by proxy.

## Evidence

Session `s_1777658707_425792000`: "what time is it now?" answered "17:35:06" at 21:05 UTC — off by 3.5 hours. Trace shows:
- `line.len() < 30` → `likely_conversational=true` → `needs_tools=false`
- Model responded 3 times with same hallucinated answer (stagnation)
- `is_trivial=true` skipped orchestration retry entirely

## Objective

Replace the character-count heuristic with the existing LLM-based route inference (`infer_route_prior` in `routing_infer.rs`), which is already defined and tested but not wired into the main execution path. When route uncertainty implies tool use, do not suppress tools because the prompt is short; allow the later tool-discovery and rust-native preference layers to choose the safest capability.

## Implementation Plan

### Phase 1: Wire annotate_and_classify into run_chat_loop

`annotate_and_classify` (in `app_chat_loop.rs:384-429`) wraps `infer_route_prior` from `routing_infer.rs:170-437`. It:
1. Calls `annotate_user_intent` for the intent annotation
2. Calls `infer_route_prior` for speech-act, workflow, and mode classification
3. Returns a `RouteDecision` with proper entropy, margin, and distribution

Wire it into the execution path at or before line 842 (where the heuristic currently sits).

```rust
// REPLACE lines 846-892 (the heuristic-driven RouteDecision construction)
// WITH:
let (rephrased_objective, route_decision) = annotate_and_classify(runtime, line).await?;

// Derive complexity from route_decision (LLM-driven, not length-driven)
let complexity = complexity_from_route(&route_decision);
let formula = formula_from_route(&route_decision);
```

### Phase 2: Ensure needs_tools is derived from LLM classification and evidence need

The LLM-based speech act already classifies CHAT vs INQUIRE vs INSTRUCT. An INQUIRE speech act (like "what time is it now?") should enable tool access even if the route is CHAT. The `complexity.needs_tools` field must be set based on the LLM's classification, not character count.

Current heuristic mapping:
- `likely_conversational=true` → `needs_tools=false`
- New: `speech_act != "CHAT" || workflow == "WORKFLOW"` → `needs_tools=true`

When a request needs current/local evidence, set `needs_tools=true` even if the route remains conversational. The orchestration layer should then prefer rust-native tools from Task 387 before shell fallback.

### Phase 3: Conservative fallback

If `infer_route_prior` fails (network error, model failure), fall back to a conservative default that ALLOWS tool access rather than blocking it:

```rust
.route("WORKFLOW")
.needs_tools(true)  // Allow tools by default when uncertain
```

Never fall back to `needs_tools=false` on error.

### Phase 4: Remove the line.len() heuristic

Delete lines 846-858. The routing decision is now entirely LLM-driven.

### Phase 5: Do not bypass tool discovery

If the LLM route is uncertain or indicates evidence need, preserve enough metadata for Task 388 to discover tools by capability. Do not collapse the turn into `reply_only` before tool discovery has a chance to run.

## Files to Modify

| File | Change |
|------|--------|
| `src/app_chat_loop.rs` | Replace heuristic routing block (lines 842-892) with `annotate_and_classify` call; remove `line.len() < 30` |
| `src/routing_infer.rs` | No changes needed (already correct) |

## Non-Scope

- Do NOT modify `routing_infer.rs` or `infer_route_prior` — they already produce correct route decisions
- Do NOT modify `src/prompt_core.rs`
- Do NOT add new keyword heuristics as fallbacks
- Do NOT remove the `extract_first_path_from_user_text` helper — it's used for workspace discovery

## Risk Assessment

- **MEDIUM**: Adding 3 extra LLM calls per user message (intent already runs, so 2 net-new: speech-act + workflow/mode) increases latency. Per objectives.md: "Accuracy over speed, with stability."
- **LOW**: `infer_route_prior` already has conservative fallback to CHAT on uncertainty, which prevents unsafe tool use
- **LOW**: The three classifiers use small profiles (few tokens), not the full model context

## Verification

```bash
cargo build
cargo test route
cargo test classify
cargo test routing
```

**Manual probe**: Send "what time is it now?" and verify:
1. Intent is correctly annotated as "The user is asking what the current time is"
2. Speech act is classified as INQUIRE (not CHAT)
3. Route allows tool access (not forced to CHAT+reply_only)
4. `date` is called via `bash` to produce correct time
5. Answer is grounded in bash output, not hallucinated

**Regression guard**: Send "hello" and verify it still routes to CHAT (short-circuit for truly conversational queries should still work via speech-act classification)
