# Task 617: Context budget exhaustion causes conversation amnesia

## Type

Bug

## Severity

Critical

## Scope

System-wide (context budget management)

## Session Evidence

**Session:** `s_1777843822_776972000`
**Model:** Huihui-Qwen3.5-4B (~4096 token context window)

The session had 13 user turns. `applied_summaries` shows 12 summaries applied. With each turn generating tool call results, tool outputs, and model responses, the total context rapidly exceeds the 4K token window.

From `trace_debug.log` — every turn:
```
trace: memory_gate_status=skip reason=missing_workspace_evidence
```
The memory gate never triggers, meaning no memory/window management is happening.

The model's context fills with turn summaries and tool call outputs. After 5-6 turns, the model's effective context window is saturated, causing:
- The model "forgets" file names from 2-3 turns ago
- The model responds with "I don't have access to conversation history"
- The model produces verbose responses that further consume context
- Each new turn summary adds more tokens, compounding the problem

`session.json` turn summaries are verbose (multi-paragraph, full sentences) — each summary potentially consumes hundreds of tokens.

## Problem

The 4K context window fills up and the system has no mechanism to:
1. Detect when the model is operating near the context limit
2. Aggressively trim/compact older messages
3. Preserve critical facts (file paths, task state) while dropping verbose content
4. Warn the user that the session needs to be reset or the model needs to be switched

The `auto_compact` module exists but isn't triggering because `memory_gate_status=skip reason=missing_workspace_evidence`.

## Root Cause Hypothesis

**Likely:**
1. The auto_compact trigger conditions don't activate because the session doesn't meet the "workspace evidence" criteria
2. The context budget tracker (`CompactTracker`) is not receiving accurate token counts for the full message history
3. Turn summaries are too verbose — they use full sentences when structured (key-value) format would save tokens
4. No mechanism to detect when the model's responses indicate context loss (amnesia detection)

## Proposed Solution

### Part A: Fix auto_compact trigger
Ensure `auto_compact` activates based on total token count (not just "workspace evidence"). If `tracker.total_tokens > ctx_limit * 70/100`, trigger compaction regardless of evidence status.

### Part B: Structured turn summaries
Change turn summary format from prose to structured key-value:
```
File: project_tmp/GEMINI.md | Action: read first 5 lines | Status: done | Output: 385 lines, Elma CLI Philosophy... | Continue: reading inside project_tmp
```
This dramatically reduces token usage per summary while preserving all critical information.

### Part C: Amnesia detection
Add a heuristic that detects when the model's response includes phrases like:
- "I don't have access to conversation history"
- "I don't know which file"
- "I can't remember"

When detected, automatically compact the context and re-inject the recent file/task context.

### Part D: Context usage display
Show current context usage in the footer (already exists as `ctx %`). When usage exceeds 80%, show a warning notice. When it exceeds 95%, suggest session reset.

Files to change:
- `src/auto_compact.rs` — fix trigger conditions
- `src/compact_tracker/mod.rs` — improve token estimation
- `src/intel_units/intel_units_turn_summary.rs` — structured summary format
- `src/tool_loop.rs` — amnesia detection injection

## Acceptance Criteria

- [ ] After 10+ turns, context usage stays below 80% through aggressive compaction
- [ ] Turn summaries use <50% of current token budget
- [ ] When model shows amnesia signs, context is auto-compacted and re-injected with relevant history
- [ ] Footer shows accurate context percentage and warns at 80%
- [ ] Long multi-turn sessions don't lose file/task continuity

## Verification Plan

- Unit test: verify structured summary format produces fewer tokens than prose
- Integration test: run 15-turn session, verify context stays below 80%
- Replay session `s_1777843822_776972000`: verify model can reference files from turn 2 in turn 10
- Measure token usage per turn summary before/after change

## Dependencies

- Task 613 (conversation forgetting) — overlapping: 613 is about semantic quality, 617 is about capacity management
- Task 619 (false completion claims) — compactions can lose task state, causing false claims

## Notes

The 4K context window is a hard constraint. The goal is not to fit unlimited history but to make the most efficient use of the available window. Structured summaries + aggressive compaction + early warning can make 15-turn sessions feasible even on 4B models.
