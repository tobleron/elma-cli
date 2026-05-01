# Task 377: Remove Trivial Chat Bypass In Orchestration Pipeline

**Status:** Pending
**Priority:** HIGHEST
**Estimated effort:** 1-2 days
**Dependencies:** Task 376 (complementary, not blocking)
**References:** AGENTS.md Rules 1, 3, objectives.md, session `s_1777658707_425792000`

## Problem

`src/app_chat_loop.rs:963-979` contains an `is_trivial` guard that skips `orchestrate_with_retries` entirely for all CHAT+reply_only queries:

```rust
let is_trivial = route_decision.route.eq_ignore_ascii_case("CHAT")
    && formula.primary.eq_ignore_ascii_case("reply_only");

let mut loop_outcome = if is_trivial || is_tool_calling_result {
    AutonomousLoopOutcome {
        program: program.clone(),
        step_results: vec![],
        final_reply: None,
        reasoning_clean: true,
    }
} else {
    orchestrate_with_retries(...).await?
};
```

When a small model produces a hallucinated answer, the retry loop never executes. The hallucinated answer is extracted directly from `Step::Respond { instructions }` at line 1017-1026 and displayed to the user with no correction opportunity.

**Evidence**: Session `s_1777658707_425792000` — the tool loop detected stagnation (3 identical `respond` calls with "17:35:06"), injected a respond abuse correction, and force-finalized. But `is_trivial=true` meant the answer was extracted directly without any retry logic. The stagnation detection and correction the tool loop performed was wasted.

## Objective

Remove the trivial bypass so ALL queries flow through `orchestrate_with_retries` or an equivalent orchestration path that can discover tools, retry, and repair. The retry loop already has mechanisms to detect failure:
- **Stagnation detection** (3 identical responds → force-finalize with correction)
- **Stale program detection** (retry program identical to prior failure)
- **Temperature escalation** (increasing temperature per attempt)
- **Strategy chains** (InspectFirst, PlanThenExecute, SafeMode, Incremental)

These mechanisms should work for CHAT queries too.

## Implementation Plan

### Phase 1: Remove is_trivial guard

Replace lines 963-979 with logic that always enters `orchestrate_with_retries` for non-tool-calling programs:

```rust
let is_tool_calling_result = program.steps.len() == 1
    && matches!(&program.steps[0], Step::Respond { instructions, .. } if !instructions.trim().is_empty());

let mut loop_outcome = if is_tool_calling_result {
    // Tool-calling pipeline produced a valid Respond step — use it directly
    AutonomousLoopOutcome {
        program: program.clone(),
        step_results: vec![],
        final_reply: None,
        reasoning_clean: true,
    }
} else {
    // All other programs (including CHAT+reply_only) go through retry orchestration
    tui.set_activity("Executing", "Executing...");
    tui.pump_ui()?;
    orchestrate_with_retries(
        &runtime.args, &runtime.client, &runtime.chat_url,
        &runtime.session, &runtime.repo, program,
        &route_decision, workflow_plan.as_ref(),
        &complexity, &scope, &formula,
        &runtime.ws, &runtime.ws_brief, &runtime.messages,
        &runtime.profiles, runtime.args.max_retries,
        runtime.args.retry_temp_step, runtime.args.max_retry_temp,
        Some(&mut tui),
    ).await?
};
```

### Phase 2: Surface retry/stagnation events in transcript

Per AGENTS.md Rule 6, stagnation detection and retry events must be visible in the transcript as collapsible rows. Add `tui.push_meta_event(...)` calls when:
- The retry loop detects stagnation
- A retry attempt produces a different strategy
- Temperature escalation kicks in

### Phase 3: Handle tool-calling pipeline stagnation in retry context

When the tool-calling pipeline produces stagnation (3 identical responds), the retry loop's `build_program_with_strategy` should detect this and rebuild with a different strategy (e.g., InspectFirst to force gathering evidence before responding).

## Files to Modify

| File | Change |
|------|--------|
| `src/app_chat_loop.rs` | Remove `is_trivial` check at lines 963-979; restructure flow |
| `src/orchestration_retry.rs` | Surface retry/stagnation events as transcript rows |

## Non-Scope

- Do NOT modify `orchestrate_with_retries` internals beyond adding transcript visibility
- Do NOT modify `src/prompt_core.rs`
- Do NOT remove `is_tool_calling_result` path — it handles correct pre-built programs
- Do NOT change the `resolve_final_text` logic at line 1028

## Risk Assessment

- **LOW**: CHAT queries already go through the tool-calling pipeline which takes one model call. Adding retry adds at most one extra call on first-attempt success
- **MEDIUM**: For genuinely trivial CHAT queries (e.g., "hello"), an extra retry call is unnecessary overhead. Mitigation: the retry loop succeeds on first attempt and returns immediately if `outcome.final_reply.is_some()` and no errors
- **LOW**: Retry logic has a max_retries cap, prevents infinite loops

## Verification

```bash
cargo build
cargo test retry
cargo test orchestration
```

**Manual probe**: Send "what time is it now?" after Task 376 is complete. Verify:
1. `date` is called via `bash`
2. Correct time is returned
3. No stagnation events in transcript (tool loop succeeds on first attempt with actual evidence)

**Stagnation guard**: If model still responds without tools, verify:
1. Stagnation is detected at attempt 3
2. Retry loop rebuilds program with InspectFirst strategy
3. Transcript shows retry attempt and strategy change
4. Final answer is grounded
