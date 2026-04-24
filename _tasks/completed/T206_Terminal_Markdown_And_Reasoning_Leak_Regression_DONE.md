# Task T206: Terminal Markdown And Reasoning Leak Regression

## Priority
P0

## Objective
Fix the live CLI regression where assistant answers render with broken markdown structure and raw model reasoning leaks into the transcript in a low-quality form.

## Why This Exists
The regression is reproducible in the real CLI and visible in session artifacts:
- session [`sessions/s_1776984861_851481000/reasoning_audit.jsonl`](/Users/r2/elma-cli/sessions/s_1776984861_851481000/reasoning_audit.jsonl) shows planner/repair responses leaking `<think>` blocks and malformed JSON-shaped output;
- session [`sessions/s_1776984861_851481000/trace_debug.log`](/Users/r2/elma-cli/sessions/s_1776984861_851481000/trace_debug.log) shows `reasoning_format_override requested=none effective=auto`;
- the live transcript screenshot shows:
  - expanded raw thinking prose in the chat history,
  - orphan bullet markers,
  - merged paragraph/list content,
  - weak terminal presentation for the final answer.

## Confirmed Root Causes
1. `reasoning_format=none` is being upgraded to `auto` for many non-JSON requests in [`src/ui/ui_chat.rs`](/Users/r2/elma-cli/src/ui/ui_chat.rs).
2. Streamed `<think>`/`<thinking>`/`<reasoning>` content is rendered as live transcript thinking in [`src/orchestration_helpers/mod.rs`](/Users/r2/elma-cli/src/orchestration_helpers/mod.rs) and [`src/claude_ui/claude_state.rs`](/Users/r2/elma-cli/src/claude_ui/claude_state.rs).
3. The Claude markdown renderer in [`src/claude_ui/claude_markdown.rs`](/Users/r2/elma-cli/src/claude_ui/claude_markdown.rs) mishandles:
   - list-item marker/content association,
   - inline code ordering,
   - soft/hard line breaks.
4. The final-answer presentation pass in [`src/app_chat_loop.rs`](/Users/r2/elma-cli/src/app_chat_loop.rs) is too weak; it strips think blocks but does not normalize malformed markdown structure.
5. Footer notification plumbing still exists in [`src/ui/ui_terminal.rs`](/Users/r2/elma-cli/src/ui/ui_terminal.rs) and [`src/claude_ui/claude_render.rs`](/Users/r2/elma-cli/src/claude_ui/claude_render.rs), which means the transcript-native telemetry migration is incomplete.

## Scope
This is a troubleshooting and closure task for real UX regressions. It is allowed to reopen parts of Task 205 and related UI tasks where the current runtime behavior does not satisfy their acceptance criteria.

## Required Fixes
1. Stop leaking raw model reasoning into user-visible transcript rows for normal assistant answers.
2. Ensure requests that explicitly ask for plain terminal text do not get upgraded from reasoning `none` to reasoning `auto`.
3. Rewrite the ratatui markdown renderer so:
   - bullet/number markers stay attached to their content line,
   - inline code preserves span order relative to surrounding text,
   - soft/hard breaks produce stable readable output,
   - paragraphs do not collapse into merged lines.
4. Remove or bypass footer notification usage for queue/mode/process notices; keep operational visibility in transcript-native rows only.
5. Strengthen the final-answer presentation contract so long answers are normalized before display.

## Non-Goals
- Do not remove transcript-native telemetry.
- Do not hide all process visibility; replace raw reasoning with curated runtime/process visibility.
- Do not reintroduce legacy boxed UI or activity rail patterns.

## Verification Requirements
1. Real `cargo run` reproduction of the math prompt no longer shows expanded low-quality raw reasoning.
2. A rendered answer containing:
   - paragraphs,
   - inline code,
   - bullets,
   - numbered items
   must display without orphan markers or merged lines.
3. PTY snapshots must cover:
   - inline-code paragraph order,
   - bullet list layout,
   - numbered list layout,
   - transcript telemetry without footer notice pollution.
4. The trace should no longer show `requested=none effective=auto` for user-facing plain-text final answers unless explicitly justified by a test-backed policy exception.

## Required Tests
- focused renderer tests for inline code ordering
- focused renderer tests for bullet and numbered list line layout
- PTY snapshot for a long assistant answer with markdown structure
- PTY snapshot for queue/telemetry in transcript rather than footer
- real CLI validation using the prompt: `Is the square root of 99 related to any golden ratio?`

## Exit Criteria
- The screenshot failure mode is no longer reproducible.
- The active runtime behavior matches Task 205’s transcript/presentation contract.
- The real CLI output is readable without relying on the user to mentally reconstruct list and paragraph structure.
