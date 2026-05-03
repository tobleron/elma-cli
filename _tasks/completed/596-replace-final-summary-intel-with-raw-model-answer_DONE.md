# Task 596: Replace Final Summary Intel With Raw Model Answer

## Session Evidence
Session `s_1777824575_8073000`: The model's `summary` tool call produced a human-readable, well-structured answer:

```markdown
# Documentation vs Source Code Comparison
## Overall Assessment: **Mostly Accurate**
I compared 7 documentation files against their corresponding source code...
**Well-aligned documentation:**
- `ARCHITECTURE.md` accurately describes the...
```

But `run_final_summary_intel` (called at `tool_loop.rs:1496`) replaced it with a hallucinated schema format:

```
- [USER_GOAL] Compare documentation against source code...
- [ASSISTANT_ACTION] Elma used shell commands (cat)...
- [OUTCOME] success - Elma concluded that all documentation is current...
```

The final answer displayed to the user was the robot-schema output, destroying semantic continuity.

## Problem
`run_final_summary_intel` runs a second model call to "summarize" the model's already-good answer, but the small model (Huihui-Qwen3.5-4B) fabricates a `[USER_GOAL]`/`[ASSISTANT_ACTION]`/`[OUTCOME]` schema instead of natural language. This is a degradation, not an improvement.

The `is_simple_turn` guard (`tool_loop.rs:1491`) already correctly uses the raw answer for simple turns. But complex turns (+2 tool calls) route through the intel unit and get their natural text replaced with robot-speak.

## Solution
**Kill `run_final_summary_intel`.** Always use the model's raw `summary` content directly as the final answer. The model already produced a complete, well-formatted answer — there's no value in having a second model call rephrase it.

Specific change in `src/tool_loop.rs:1479-1521`:
- Remove the `is_simple_turn` gate
- Remove the `run_final_summary_intel` call
- Use `raw_content` directly as `final_answer`

Optionally, apply lightweight post-processing to strip thinking blocks (already done) and normalize whitespace, but do NOT run a second model call.
