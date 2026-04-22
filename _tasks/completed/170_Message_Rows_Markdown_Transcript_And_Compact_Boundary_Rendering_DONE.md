# Task 170: Message Rows, Markdown, Transcript, And Compact Boundary Rendering

## Continuation Checklist
- [ ] Re-read this task and all linked source/task references before editing.
- [ ] Confirm the task is still valid against current `_tasks/TASKS.md`, `AGENTS.md`, and active master plans.
- [ ] Move or keep this task in `_tasks/active/` before implementation work begins.
- [ ] Inspect the current code/config/docs touched by this task and note any drift from the written plan.
- [ ] Implement the smallest coherent change set that satisfies the next unchecked item.
- [ ] Add or update focused tests, probes, fixtures, or snapshots for the changed behavior.
- [ ] Run `cargo fmt --check` and fix formatting issues.
- [ ] Run `cargo build` and resolve all build errors or warnings introduced by this task.
- [ ] Run targeted `cargo test` commands and any task-specific probes listed below.
- [ ] Run real CLI or pseudo-terminal verification for any user-facing behavior.
- [ ] Record completed work, verification output, and remaining gaps in this task before stopping.
- [ ] Ask for sign-off before moving this task to `_tasks/completed/`.

## Status
Completed.

## Completed Work
- [x] Integrated `ratatui` for robust, flicker-free terminal rendering.
- [x] Added `to_ratatui_lines` and `render_ratatui` methods to `ClaudeMessage` and `ClaudeTranscript`.
- [x] Implemented `ClaudeMarkdown` renderer using theme tokens and `ratatui` primitives.
- [x] Added compact boundary row and expanded/collapsed transcript mode.
- [x] Verified through `cargo test` and `ui_parity` snapshot testing.

## Objective
Implement Claude Code-style message row rendering for users, assistant text, thinking, tool messages, system notices, compact summaries, and transcript expansion.

## Claude Source References
- `components/Messages.tsx`
- `components/Message.tsx`
- `components/MessageRow.tsx`
- `components/messages/AssistantTextMessage.tsx`
- `components/messages/AssistantThinkingMessage.tsx`
- `components/CompactSummary.tsx`
- `components/messages/CompactBoundaryMessage.tsx`
- `components/Markdown.tsx`

## Message Row Requirements
Implement visible row behavior for:

- User prompt rows with `>` prefix.
- Assistant rows with `●` prefix and markdown body.
- Thinking rows with `∴ Thinking` collapsed by default and expanded in transcript/verbose mode.
- Tool rows delegated to Task 172 but represented in the same transcript model.
- Compact summary rows with `●` and summary metadata.
- Compact boundary rows using `✻ Conversation compacted (ctrl+o for history)` style behavior.
- System/status notices only where Claude source shows analogous notices.

## Markdown Requirements
Match Claude Code's terminal markdown behavior where source-observable:

- Headers.
- Bold.
- Italic.
- Inline code.
- Links/path-like text.
- Lists.
- Numbered lists.
- Blockquotes with vertical bar style.
- Horizontal rules.
- Code fences with language-aware highlighting.
- Tables if present in assistant output.
- Safe wrapping around ANSI and wide Unicode characters.

Use existing `syntect` if sufficient. Add `pulldown-cmark` or another parser if the current markdown implementation cannot provide the needed structure safely.

## Transcript And Expansion
- Default view should be compact and Claude-like.
- Ctrl-O or configured transcript mode should expose hidden details such as thinking content, compact history, and tool outputs according to Claude behavior.
- Expanded mode must not destroy scroll position.
- The UI must indicate when history is available without adding permanent Elma-specific chrome.

## Data Model
Refactor `TranscriptItem` or replace it with a model that can represent:

- Stable IDs.
- Parent/child grouping for tool sequences.
- Collapsed and expanded text.
- Streaming state.
- Compact boundaries.
- Snapshot-test-friendly deterministic ordering.

## Acceptance Criteria
- Assistant responses render with Claude-like `●` rows and markdown formatting.
- Thinking rows appear with the Claude `∴ Thinking` convention, not Elma's old `~` convention.
- Compact boundary and compact summary rows match the source-observed semantics.
- Transcript expansion can be toggled and snapshot-tested.
- Rendering uses the Pink/Cyan theme tokens from Task 168.

## Verification
Run:

```bash
cargo fmt --check
cargo build
cargo test markdown -- --nocapture
cargo test transcript_rendering -- --nocapture
cargo test ui_parity_messages -- --nocapture
./ui_parity_probe.sh --fixture assistant-markdown
./ui_parity_probe.sh --fixture thinking-collapsed-expanded
./ui_parity_probe.sh --fixture compact-boundary
```

The final probes must render through the real CLI pseudo-terminal, not just through pure functions.
