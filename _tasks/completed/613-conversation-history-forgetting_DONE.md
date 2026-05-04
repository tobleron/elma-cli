# Task 613: Conversation history forgetting (context amnesia)

## Type

Bug

## Severity

Critical

## Scope

System-wide (conversation context / effective history)

## Session Evidence

**Session:** `s_1777843822_776972000`
**Model:** Huihui-Qwen3.5-4B (3.5B params, ~4K context window)

Turn 5 ("the one I just asked you about in my previous prompt?"):
The model literally responded that it cannot remember the previous prompt. From `session.json` turn 5 summary:
> "the model responded that it lacks access to conversation history or previous prompts in its current context"

Turn 4 ("show me the first 5 lines of that file"):
Model used 0 tool calls, produced 181 chars of text explaining it doesn't know which file — it forgot that the previous turn was about GEMINI.md.

The session had 13 user turns with `applied_summaries: [0,1,2,3,4,5,6,7,8,9,10,11]` — 12 conversation summaries applied. With a 4B model's limited context window, the accumulated turn context eventually exceeds available tokens, forcing summarization that loses critical detail.

From `trace_debug.log`:
```
trace: memory_gate_status=skip reason=missing_workspace_evidence
```
This appears on every turn — the memory gate never activates.

## Problem

The 4B model has a very limited context window (~4096 tokens). After 5-6 turns with tool call results, the conversation history exceeds the context budget. Summarization compresses older turns but loses key details (like file names, paths, task state). The model then starts hallucinating or responding that it "doesn't have access to conversation history."

The effective_history system and turn summaries are supposed to carry forward key context, but they're either not being used effectively or the summaries lose too much information.

## Root Cause Hypothesis

**Likely:**
1. The context window of the 4B model (~4K tokens) fills up faster than summarization can compress
2. Turn summaries lose critical information (file paths, task state) — semantic compression vs. lossless reference
3. The `effective_history` or evidence injection mechanism doesn't carry forward the essential "what file are we working on" context
4. After summarization, the model loses the thread of multi-turn tasks because the summaries don't include enough specifics

## Proposed Solution

### Part A: Improve turn summary fidelity
In `src/intel_units/intel_units_turn_summary.rs`, enhance the turn summary prompt to require:
- Exact file paths worked on
- Exact command outputs (key findings)
- Task completion state (done/partial/failed)
- Connection to previous turn (what file/context carries forward)

### Part B: Inject working memory before each turn
Before each tool loop iteration, inject a "Working Memory" block listing:
- The current active file path(s)
- The current task objective
- Last tool output summary
- Key facts discovered so far

This keeps the model grounded even when earlier context is summarized away.

### Part C: Detect context amnesia
Add detection for when the model responds with "I don't have access to history" or "I don't know which file" — immediately inject the relevant context from turn summaries rather than letting the model flounder.

Files to change:
- `src/intel_units/intel_units_turn_summary.rs` — improve summary fidelity
- `src/effective_history.rs` — ensure file paths and task state carry forward
- `src/tool_loop.rs` — inject working memory before each model turn

## Acceptance Criteria

- [ ] After 10+ turns, the model can still reference file names from 2-3 turns ago
- [ ] Turn summaries include exact file paths and key command outputs
- [ ] Model never responds with "I don't have access to conversation history" when the information is in previous turns
- [ ] Multi-turn file operations (find file → read file → edit file) don't lose the file reference

## Verification Plan

- Replay session `s_1777843822_776972000` with improved summarization — verify turn 5 doesn't lose GEMINI.md reference
- Synthetic test: run 10-turn sequence referencing files from early turns, verify model stays grounded
- Check turn summaries for inclusion of file paths

## Dependencies

- Task 617 (context budget exhaustion) — overlapping but distinct: this is about semantic quality, 617 is about raw capacity
- Task 619 (false completion claims) — summarization quality also affects completion accuracy

## Notes

The 4B model's 4K context window is a hard constraint. Even perfect summarization can't carry infinite context. The solution should prioritize keeping the most salient context (file paths, task state, recent outputs) and aggressively drop verbose tool outputs and conversational filler.
