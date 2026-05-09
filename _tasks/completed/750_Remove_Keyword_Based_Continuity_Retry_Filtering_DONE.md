# Task 750: Remove Keyword-Based Continuity Retry Filtering

## Type

Architecture / Rule 1 Violation / Reliability

## Severity

High

## Scope

Finalization, continuity retry, tool loop

## Problem

`src/app_chat_loop.rs` (lines 1079-1088) implements continuity retry validation using keyword matching against XML-like tags:

```rust
let is_valid_answer = trimmed.len() >= 20
    && !trimmed.starts_with('<')
    && !improved.contains("<search_files")
    && !improved.contains("<read ")
    && !improved.contains("<write ")
    && !improved.contains("<glob ")
    && !improved.contains("<shell")
    && !improved.contains("<bash");
```

This is a **Rule 1 violation** (no word-based routing/classification) and a **JSON architecture violation**.

Problems:
1. **Keyword matching**: uses `contains("<read ")` to detect tool proposals
2. **XML tags in JSON tool-calling system**: the codebase uses JSON tool calling (`{"name": "read", "arguments": {...}}`), but this filter checks for XML-like tags (`<read `). These tags don't exist in the JSON tool-calling pipeline.
3. **False positives**: a valid answer containing "The shell command `<bash>...`" would be rejected
4. **False negatives**: a tool proposal in JSON format (`{"name": "read"}`) would pass the filter
5. **Brittle**: adding a new tool requires updating this filter
6. **Comment admits the problem**: "Small models often respond to 'expand' prompts with tool proposals instead of text"
   - But the fix (keyword filtering) is the wrong solution
   - The right solution is better prompt engineering or a dedicated intel unit

This violates:
- **Rule 1**: no word-based triggers
- **Rule 3**: decomposition beats keyword lists
- **Rule 7**: principle-first, not example-driven
- **Rule 8**: canonical prompt stability — this is a brittle runtime patch

## Root Cause

The continuity retry was added to prevent small models from emitting tool calls when asked for text. Instead of fixing the prompt or using a structured output constraint, a keyword filter was added as a quick patch.

## Proposed Solution

### Phase 1 — Replace with structured output validation

The continuity retry request should enforce that the model returns **text only**, not tool calls:

1. Use the JSON tool-calling API but set `tools: None` or `tool_choice: "none"` for the retry request
2. If the API doesn't support `tool_choice: "none"`, use a system prompt that says "Respond with text only. Do not call tools."
3. After receiving the response, validate that it is text (not a tool call) by checking the API response structure, not the content string

### Phase 2 — Delete keyword filter

1. Delete the `is_valid_answer` block with all `contains("<...")` checks
2. Replace with:
   ```rust
   let is_valid_answer = response.choices.get(0)
       .and_then(|c| c.message.content.as_ref())
       .is_some()
       && response.choices.get(0)
       .map(|c| c.message.tool_calls.is_none())
       .unwrap_or(true);
   ```

   (Adjust based on actual API response structure.)

### Phase 3 — Harden the retry prompt

1. The continuity retry system prompt should explicitly say:
   - "You are in text-only mode."
   - "Do not propose tool calls."
   - "Use the evidence already gathered to answer the user."
2. Set temperature to 0.3 or lower for the retry to reduce creative tool proposals
3. Set `max_tokens` to a reasonable limit to prevent runaway output

### Phase 4 — Fallback on tool proposal

If the model still proposes a tool call during retry:
1. Reject the retry output
2. Keep the original answer
3. Trace: "continuity_retry_rejected: model proposed tools in text-only mode"

## Acceptance Criteria

- [ ] No `contains("<read ")`, `contains("<write ")`, etc. in `app_chat_loop.rs`
- [ ] Continuity retry uses API-level text-only enforcement (tool_choice or no tools)
- [ ] Retry validation checks response structure, not content substrings
- [ ] Retry prompt is hardened for text-only responses
- [ ] If model proposes tools during retry, original answer is preserved
- [ ] `cargo build && cargo test` passes

## Verification Plan

- `grep -n "contains(\"<" src/app_chat_loop.rs` → no matches
- Unit test: mock response with text content → accepted
- Unit test: mock response with tool calls → rejected, original kept
- Integration test: continuity retry produces text, not tool proposals

## Dependencies

- `src/app_chat_loop.rs` (primary site)
- `src/llm_config.rs` (tool_choice configuration)
- `src/llm_provider.rs` (API request building)

## Notes

This is another example of "patching symptoms instead of fixing the system." The docs say:

> "Do: decompose: add a focused intermediary intel unit, tighten narrative context, or reduce cognitive load per step."

The correct fix is to use the API's text-only mode, not to scan for XML tags that shouldn't exist in a JSON tool-calling system.

If the API doesn't support tool_choice, the prompt must be tightened. The current retry prompt says:

> "Please provide a more complete answer. Do NOT call any tools — use the evidence you already gathered."

This is good, but it should be in the system prompt, not the user message, and it should be paired with `tools: None` in the request.

The XML tag filtering is especially egregious because it suggests the codebase is still carrying baggage from a previous DSL/XML action protocol. The docs say:

> "Current active architecture is strict JSON/tool-calling plus compact intel-unit JSON. Do not revive DSL action protocols."

Delete the XML tag checks as part of removing DSL baggage.
