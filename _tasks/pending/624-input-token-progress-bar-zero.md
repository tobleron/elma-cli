# Task 624: Input Token Progress Bar Always Shows 0%

## Type
Bug

## Severity
Medium

## Scope
System-wide

## Session Evidence
- Session: s_1777932314_635562000 (2026-05-05)
- User reports: "the in token generation is staying at 0 at all times"
- Footer shows input tokens progress bar at 0% regardless of conversation length
- Primary LLM response does not report usage statistics

## Problem
The input token progress bar uses formula `input_tokens / token_max` where:
1. `input_tokens` is estimated from `content.len() / 2` per message
2. `token_max` is the model's `max_response_tokens_cap` (default 16384)

For typical messages (~50-200 estimated input tokens), the fraction is 0.3-1.2%, which renders as 0% on the progress bar.

Additionally, the model server does not return `usage.prompt_tokens` in the API response, so actual token counts cannot be obtained.

## Root Cause Hypothesis
Confirmed: The progress bar's denominator (`max_response_tokens_cap`, 16384) is much larger than typical input token counts (~50-200). The fraction rounds to 0% visually.

## Proposed Solution
1. Use the model's context window size as max for input tokens (e.g., 4096 or 8192 for typical models)
2. Or scale the progress bar differently for input tokens (use a separate max value)
3. Estimate more accurately: include system prompt, conversation history, tool results
4. Files: `src/claude_ui/claude_render.rs` (line ~1940), `src/ui/ui_terminal.rs`

## Acceptance Criteria
- [ ] Input token progress bar shows non-zero values for any conversation
- [ ] Progress bar increases as conversation grows
- [ ] Max value is reasonable (not 16384 for input tokens)

## Verification Plan
- Start elma-cli, send a message
- Check footer for input token progress bar showing > 0%
- Send multiple messages, verify progress increases

## Dependencies
None.
