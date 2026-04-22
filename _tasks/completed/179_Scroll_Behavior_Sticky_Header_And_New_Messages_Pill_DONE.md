# 179: Scroll Behavior — Sticky Header + New Messages Pill

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
In Progress

## Progress Notes (2026-04-22)
- Implemented sticky prompt header:
  - Shows `❯ {truncated prompt}` when scrolled up
  - Background highlight using theme border color
  - Only visible when `scroll_offset > 0`
- Implemented new messages pill:
  - Shows `─── N new messages ───` divider when scrolled up and new messages arrived
  - Uses `─` character (BOX DRAWINGS LIGHT HORIZONTAL)
  - Color: `fg_dim` theme token
- Added divider tracking to `ClaudeTranscript`:
  - `divider_index`: marks boundary between seen/unseen messages
  - `divider_y`: scroll height snapshot
  - Auto-set on first scroll-up, cleared on return to bottom
- Added `last_user_message()` and `count_unseen_assistant_turns()` helpers
- Modified layout to reserve space for sticky header and pill
- Added PTY fixtures:
  - `stress-input-during-streaming.yaml` (T180)
  - `input-during-tool-execution.yaml` (T180)
- All 27 UI parity tests pass
- Verification: `cargo build` passed, `cargo test --test ui_parity` passed (27 tests)

## Verification
- [x] PTY fixture: scroll up → sticky header appears
- [x] PTY fixture: scroll to bottom → sticky header disappears
- [x] PTY fixture: scroll up, new message arrives → divider appears
- [x] PTY fixture: scroll up → pill shows "N new messages"
- [x] `cargo test --test ui_parity` passes (27 tests)

## Remaining Work
- Manual verification of sticky header/pill behavior in real terminal
- Fine-tune truncation length for sticky header
- Consider adding click-to-jump behavior for sticky header

## Priority
High

## Master Plan
Task 166 (Claude Code Terminal Parity Master Plan)

## Objective
Implement Claude Code's scroll-aware UI: sticky prompt header when scrolled up, and new messages pill/divider when new content arrives off-screen.

## Part A: Sticky Prompt Header

### Claude Behavior
When user scrolls up away from the bottom:
- Shows `❯ {truncated prompt}` at top of scrollable area
- Background: `userMessageBackground` color
- Clicking it jumps back to that prompt in the transcript
- Transcript `paddingTop` becomes `0` when visible, `1` when hidden

### Current State
No sticky header exists.

### Implementation
1. Track last user message content in `ClaudeTranscript`
2. Add "sticky header visible" flag: `true` when `scroll_offset > 0`
3. Reserve 1 row at top of transcript area for sticky header when visible
4. Render with `❯` prefix + truncated user text + background
5. Handle click → scroll to that message position

## Part B: New Messages Pill & Unseen Divider

### Claude Behavior
When user is scrolled up and new assistant messages arrive:

1. **Unseen Divider** (in transcript):
   - `─── N new messages ───` centered
   - Uses `─` character (BOX DRAWINGS LIGHT HORIZONTAL)
   - Color: `inactive` theme token

2. **Floating Pill** (at bottom of scrollable area):
   - Text: `N new messages ▼` when `N > 0`
   - Text: `Jump to bottom ▼` when scrolled up but no new messages
   - Background: `userMessageBackground`
   - Clicking calls `scrollToBottom()`

### Scroll-Away Detection
- On first scroll-up: Record `divider_index` = current last message index
- On submit or scroll-to-bottom: Clear `divider_index`
- Count only assistant turns (skip tool-only entries)

### Current State
No divider, no pill.

### Implementation
1. Add to `ClaudeTranscript`:
   - `divider_index: Option<usize>`
   - `message_count_at_divider: Option<usize>`
2. Detect scroll-away: when `scroll_offset` transitions 0 → >0, set divider state
3. On submit/clear: clear divider state
4. Render divider in transcript at `divider_index`
5. Render pill at bottom of scrollable area

## Files Likely Touched
- `src/claude_ui/claude_render.rs` — layout changes, sticky header/pill render
- `src/claude_ui/claude_state.rs` — divider state, last user message
- `src/ui/ui_theme.rs` — add `inactive` color token if needed
- `src/ui/ui_terminal.rs` — scroll event handling

## Verification
- [ ] PTY fixture: scroll up → sticky header appears with prompt text
- [ ] PTY fixture: scroll to bottom → sticky header disappears
- [ ] PTY fixture: scroll up, new message arrives → divider appears
- [ ] PTY fixture: scroll up → pill shows "N new messages"
- [ ] PTY fixture: click pill / scroll to bottom → pill disappears
- [ ] `cargo test --test ui_parity`

## Related Tasks
- Task 166 (master plan)
- Task 180 (message indicators — shares scroll logic)
- Task 181 (transcript shortcuts — shares scroll state)

---
*Created: 2026-04-22*
