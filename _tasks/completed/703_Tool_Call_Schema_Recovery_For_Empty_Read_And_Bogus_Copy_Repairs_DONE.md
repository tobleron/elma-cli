# Task 703: Tool Call Schema Recovery For Empty Read And Bogus Copy Repairs

## Type

Tool Reliability / Dense Model Support

## Severity

High

## Session Evidence

Round 3 sessions repeatedly show invalid or badly repaired tool calls:

- Prompt 01: `TOOL_VALIDATION_ERROR tool=read error=filePath: required field 'filePath' is missing`
- Prompt 02: repeated `read:` duplicate failures before repair to `src/project_memory.rs`
- Prompt 05: repeated empty `read` failures led to `respond_abuse`
- Prompt 08: `copy` was repaired with invalid paths like `ago)` and `Copied`

## Problem

Dense models often emit partial tool calls or wrong argument keys. Current repair sometimes helps (`path` to `filePath`) but also injects non-path words from prior output into copy calls. This wastes iterations and can push the loop into respond abuse or bad shell fallbacks.

## Proposed Solution

Strengthen tool schema recovery with typed argument contracts and evidence-derived path validation.

Likely source areas:

- `src/tool_repair.rs`
- `src/tool_calling.rs`
- `src/tools/validation.rs`
- `src/tool_loop.rs`
- `src/provider_recovery.rs`

Requirements:

- For `read`, if no path is present, do not count repeated empty reads as a meaningful new attempt.
- Provide the model a compact tool-specific correction packet with required fields and last valid candidate paths.
- For `copy`, only inject paths that exist and came from structured path-bearing tool outputs, never arbitrary words from shell text.
- Add per-tool repair allowlists for acceptable argument aliases.
- After two failed schema repairs for the same tool, force a strategy shift or finalization rather than continuing the same invalid call.

## Acceptance Criteria

- [ ] Empty `read` calls are corrected or stopped within one retry.
- [ ] `copy` repair never injects non-existing strings such as `Copied` or `ago)`.
- [ ] Tool repair trace records source of repaired arguments.
- [ ] Invalid repeated tool calls do not consume a full continuation budget.

## Verification Plan

Replay prompts 01, 02, 05, and 08. Add unit tests for `tool_repair` covering empty read, path alias mapping, and copy repair candidate filtering.

