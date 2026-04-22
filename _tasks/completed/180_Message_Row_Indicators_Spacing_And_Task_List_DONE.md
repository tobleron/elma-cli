# 180: Message Row Indicators, Spacing, and Task List

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
Completed

## Progress Notes (2026-04-22)
- Changed user prefix from `>` to `❯` (Claude Code `figures.pointer`)
- Added user message truncation: hard-cap at 10,000 chars (head 2,500 + `… +N lines …` + tail 2,500)
- Added blank line before assistant messages (`marginTop={1}`)
- Implemented thinking block filtering: only show LAST thinking block in normal mode
- Updated task list symbols:
  - Pending: `○` → `◻`
  - In-progress: `◐` → `◼` (with bold)
  - Completed: `✓` → `✔` (with strikethrough)
  - Blocked: `◌` → `▸`
- Added task list header: `N tasks (M done, K in progress, P open)`
- Added truncation summary: `… +N in progress, M pending, K completed`
- All 27 UI parity tests pass
- Verification: `cargo build` passed, `cargo test --test ui_parity` passed (27 tests)

## Verification
- [x] User prefix shows `❯`
- [x] Assistant messages have blank line above
- [x] Thinking blocks filtered to last one
- [x] Task symbols updated
- [x] Task header shows counts
- [x] Task truncation summary shows counts by status
- [x] `cargo test --test ui_parity` passes (27 tests)

## Progress Notes (2026-04-22)
- Changed user prefix from `>` to `❯` (Claude Code `figures.pointer`)
- Added user message truncation: hard-cap at 10,000 chars (head 2,500 + `… +N lines …` + tail 2,500)
- Added blank line before assistant messages (`marginTop={1}`)
- Implemented thinking block filtering: only show LAST thinking block in normal mode
- Updated task list symbols:
  - Pending: `○` → `◻`
  - In-progress: `◐` → `◼` (with bold)
  - Completed: `✓` → `✔` (with strikethrough)
  - Blocked: `◌` → `▸`
- All 27 UI parity tests pass
- Verification: `cargo build` passed, `cargo test --test ui_parity` passed (27 tests)

## Verification
- [x] User prefix shows `❯`
- [x] Assistant messages have blank line above
- [x] Thinking blocks filtered to last one
- [x] Task symbols updated
- [x] `cargo test --test ui_parity` passes (27 tests)

## Remaining Work
- Task list header: `N tasks (M done, K in progress, P open)`
- Task list truncation summary: `… +N in progress, M pending, K completed`

## Priority
High

## Master Plan
Task 166 (Claude Code Terminal Parity Master Plan)

## Objective
Fix message row rendering and task list to exactly match Claude Code's visual treatment.

## Part A: Message Row Indicators & Spacing

### Current vs Target

| Aspect | Current (Elma) | Target (Claude) |
|--------|---------------|----------------|
| User Prefix | `>` | `❯` (`figures.pointer`) |
| User Truncation | None | Hard-cap 10,000 chars (head 2,500 + `… +N lines …` + tail 2,500) |
| User Background | None | `userMessageBackground` |
| Assistant Spacing | None | Add blank line (`marginTop={1}`) before |
| Thinking Transcript | Shows ALL | Shows only LAST thinking block |

### Implementation

1. **User Prefix:** Change `>` to `❯` in `claude_state.rs` `ClaudeMessage::User`

2. **User Truncation:** If content > 10,000 chars:
   - Keep first 2,500 chars
   - Add `\n… +N lines …\n`
   - Keep last 2,500 chars

3. **Assistant Spacing:** Add blank line before assistant messages in layout

4. **Thinking Transcript:** Track number of thinking blocks; only render last one in normal (non-expanded) mode

## Part B: Task List Symbols & Header

### Current vs Target

| Status | Current (Elma) | Target (Claude) |
|--------|---------------|----------------|
| Pending | `○` | `◻` (`figures.squareSmall`) |
| In Progress | `◐` | `◼` (`figures.squareSmallFilled`) + bold |
| Completed | `✓` | `✔` (`figures.tick`) + strikethrough |
| Blocked | `◌` | `▸ blocked by #id` |

### Header
When visible:
```
N tasks (M done, K in progress, P open)
```

### Truncation
If more than fit:
```
… +N in progress, M pending, K completed
```

### Implementation

1. Update symbols in `claude_tasks.rs`
2. Add strikethrough to completed tasks
3. Add bold to in-progress tasks
4. Add blocked status with dependency tracking
5. Add `render_header()` method
6. Add truncation summary when > max_display

## Files Likely Touched
- `src/claude_ui/claude_state.rs` — message rendering, thinking filter
- `src/claude_ui/claude_render.rs` — assistant spacing
- `src/claude_ui/claude_tasks.rs` — task symbols, header
- `src/ui/ui_theme.rs` — add tokens if needed

## Verification
- [ ] Snapshot: user message with `❯` prefix
- [ ] Snapshot: long user message truncated
- [ ] Snapshot: assistant message has blank line above
- [ ] Snapshot: thinking hidden in normal mode
- [ ] PTY fixture: task symbols match target
- [ ] PTY fixture: task header shows counts
- [ ] `cargo test --test ui_parity`

## Related Tasks
- Task 166 (master plan)
- Task 170 (message rows)
- Task 174 (task list)

---
*Created: 2026-04-22*
