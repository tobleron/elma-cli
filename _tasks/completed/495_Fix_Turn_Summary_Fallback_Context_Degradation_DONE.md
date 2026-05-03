# Task 495: Fix Turn Summary Fallback Context Degradation

**Status:** pending
**Priority:** HIGH
**Source:** Session s_1777735825_94786000 deep trace analysis (2026-05-02)
**Related:** Task 456 (EvidenceLedger staleness), Task 470 (event_log)

## Evidence From Session

- Session `s_1777735825_94786000` had all 3 turn summaries fail with "URL error: relative URL without a base"
- The `turn_summary` profile had empty `base_url` (FIXED in this commit via profile sync)
- Fallback summaries all produced: `User asked: "X". Elma responded (formula: unknown, tools: ) but the summary generation failed.`
- `applied_summaries: [0, 1]` means these degraded summaries were used as context compression for turns 1 and 2
- Tools are lost: `tools_used: []`, `tool_call_count: 0` in all fallback summaries
- This cascaded: turn 0's failed summary fed into turn 1's context, then turn 1's failed summary fed into turn 2's context

## Problem

When `TurnSummaryUnit.execute()` fails (model timeout, URL error, parse error), the `fallback()` method produces a useless string that destroys context quality when applied as a conversation summary. Successive failures compound this degradation.

The system applies these summaries into the conversation history via `applied_summaries` in session.json, meaning future turns receive "summary generation failed" instead of actual conversation content.

## Root Cause

`TurnSummaryUnit::fallback()` at `src/intel_units/intel_units_turn_summary.rs:122-148` only captures 120 chars of user message + error, losing all tool results, assistant responses, and evidence collected during the turn.

```rust
fn fallback(&self, context: &IntelContext, error: &str) -> Result<IntelOutput> {
    let user_msg = context.user_message.chars().take(120).collect::<String>();
    let tools = context.extra("tools_used").and_then(|v| v.as_str()).unwrap_or("none");
    let formula = context.extra("formula").and_then(|v| v.as_str()).unwrap_or("unknown");
    Ok(IntelOutput::fallback(
        self.name(),
        serde_json::json!({
            "summary_narrative": format!("User asked: \"{user_msg}\". Elma responded (formula: {formula}, tools: {tools}) but the summary generation failed."),
            "status_category": "partial",
            "noteworthy": false,
            "tools_used": [],   // <-- DESTROYS real tool names
            "tool_call_count": 0, // <-- DESTROYS real count
            "errors": [error.to_string()],
            "artifacts_created": [],
        }),
        &format!("turn summary failed: {}", error),
    ))
}
```

## Fix

### Phase 1: Improve fallback to preserve actual turn data
- Instead of throwing away tool data, capture what's available from context extras
- Build a summary narrative from actual user request + tool calls + final response + evidence
- Preserve `tools_used`, `tool_call_count` from context extras

### Phase 2: Auto-compaction should skip degraded summaries
- When a summary has `status_category: "partial"` and `tools_used: []`, the compaction system should NOT apply it as conversation context
- Instead, keep the raw messages for that turn (don't compact)
- Only apply summaries with `noteworthy: false` when they have actual content

### Phase 3: Fallback quality threshold
- If the fallback summary is essentially "I don't know what happened", mark it with a flag (`degraded: true`)
- The auto-compaction system checks this flag and refuses to apply degraded summaries

## Implementation Plan

1. Modify `TurnSummaryUnit::fallback()` to construct a useful narrative from available context:
   - Include final_text from context extras (truncated)
   - Include actual tools_used array from context extras  
   - Include actual tool_call_count
   - Include step_results summary if available
   - Output format: `"User asked: \"{request}\". Used {count} tool(s): {tools}. Outcome: {final_excerpt}"`

2. Add `String::from(context.user_message)` extraction to the fallback to get full user request

3. In `auto_compact.rs`, add a check: if a summary is `partial` AND has `tools_used: []`, skip compaction for that turn

4. Add tests:
   - Test fallback with full context (user message + tools + final_text)
   - Test fallback with minimal context
   - Test auto-compact skips degraded summaries

## Success Criteria

- [ ] Fallback summary includes user request, tools used, and final response
- [ ] `tools_used` and `tool_call_count` are preserved from context extras
- [ ] Auto-compaction skips summaries where `tools_used` is empty and status is partial
- [ ] Future turns receive useful context even when summary model is unavailable
- [ ] Tests pass for degraded summary skip logic

## Anti-Patterns To Avoid

- Do not call the model again in the fallback (defeats the purpose)
- Do not leave the field empty - always produce a narrative string
- Do not silently skip summary - the flag approach makes it explicit
