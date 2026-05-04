# Task 623: Thought Summaries Not Appearing For Models Using &lt;think&gt; Tags

## Type
Bug

## Severity
High

## Scope
System-wide

## Session Evidence
- Session: s_1777932314_635562000 (2026-05-05)
- Request: "what is the square root of 999?"
- Trace shows auxiliary model call succeeds (HTTP 200, 2000 bytes received, parsed successfully at line 46-49)
- Summary does NOT appear on right side panel
- reasoning_audit.jsonl shows primary LLM wraps responses in `<think>` tags
- `has_reasoning` is `false` (model sends thinking inline, not via reasoning_content field)

## Problem
When the primary LLM wraps its thinking in `<think>` tags directly in the content field (instead of using a separate `reasoning_content` field), `combined_thinking` may be empty because:
1. `turn.reasoning_content` is None (model doesn't use separate reasoning field)
2. `turn.thinking_content` may not capture think-tag content from the assistant message content

When `combined_thinking` is empty, the auxiliary summarizer is never called, and no summary appears on the right panel.

## Root Cause Hypothesis
Likely: `turn.thinking_content` is populated from `thinking_content::extract_thinking()` which may only process specific model formats. Models that emit think-content inline in `message.content` may not have their thinking extracted to `turn.thinking_content`.

## Proposed Solution
1. Inspect `src/thinking_content.rs` and trace how `turn.thinking_content` is populated
2. Ensure thinking content is extracted from ALL model formats (not just specific sentinel-reasoning formats)
3. Add a fallback: if `turn.thinking_content` is empty but `content` contains `<think>` tags, extract the think content
4. Files likely to change: `src/thinking_content.rs`, `src/tool_loop.rs`

## Acceptance Criteria
- [ ] Summaries appear for models that use `<think>` tags inline
- [ ] First prompt always shows a summary if any thinking content exists
- [ ] No regression for models that use separate `reasoning_content` field

## Verification Plan
- Run elma-cli against a model that emits `<think>` in content (tested with Huihui-Qwen3.5-4B)
- Send a simple prompt like "hello"
- Verify summary appears on right panel after response
- Check trace_debug.log for "push_thought_summary" entries

## Dependencies
None.
