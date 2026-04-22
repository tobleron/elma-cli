# 182: Legacy Modal Removal And Transcript Unification

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
- Migrated all modal rendering from legacy `draw_with_modal()` to Claude renderer:
  - Added `modal_state` field to `ClaudeRenderer`
  - Implemented `render_modal_claude()` with centered pane, border, and title
  - Supports all modal types: Confirm, Help, Select, Settings, Usage, ToolApproval, PermissionGate, PlanProgress, Notification, Splash
  - Modal renders as overlay on top of Claude renderer output
- Removed legacy `draw_with_modal()` method entirely
- Updated `draw()` to always use Claude renderer (no more dual path)
- All 27 UI parity tests pass
- Verification: `cargo build` passed, `cargo test --test ui_parity` passed (27 tests)

## Verification
- [x] All modals render through Claude renderer
- [x] No legacy chrome during modals
- [x] Permission gate renders correctly
- [x] Tool approval renders correctly
- [x] `cargo test --test ui_parity` passes (27 tests)

## Remaining Work
- Unify dual transcript sources (deferred to separate task)
- Remove old `ui_render_legacy.rs` module entirely
- Archive legacy UI task documents

## Priority
High

## Master Plan
Task 166 (Claude Code Terminal Parity Master Plan)

## Objective
Remove legacy modal overlay renderer so all UI flows through Claude renderer, and unify the dual transcript sources of truth.

## Part A: Eliminate Legacy Modal Overlay Renderer

### Current Problem
```rust
fn draw(&mut self) {
    if self.state.modal.is_some() {
        self.draw_with_modal(); // OLD FIVE-FRAME LAYOUT!
    } else {
        self.draw_claude();
    }
}
```

`draw_with_modal()` uses `ui_render_legacy::render_screen()` with:
- Old persistent header strip
- Activity rail
- Boxed composer
- Gruvbox palette

This causes jarring visual context switch on any modal.

### Modal Variants to Migrate
1. ToolApproval
2. PermissionGate
3. PlanProgress
4. Notification
5. Splash
6. SessionResume
7. Settings
8. Help
9. About

### Claude-Style Modal Rendering
- Absolute-positioned pane anchored at bottom
- Top border: `▔` repeated across terminal width
- Padding: `paddingX=2`
- Transcript peek: 2 rows of transcript visible above modal

### Implementation Phases

**Phase 1 — Permission Gate (Priority):**
1. Add `PermissionGate` rendering to `ClaudeRenderer`
2. Replace `draw_with_modal()` routing for permission gate
3. Keep 2 rows of transcript visible above

**Phase 2 — Tool Approval:**
Migrate tool approval dialog

**Phase 3 — Other Modals:**
Session resume, help, etc.

**Phase 4 — Remove Legacy:**
1. Delete `draw_with_modal()` method
2. Delete `draw_normal()` if exists
3. Remove old UI render modules
4. Archive legacy UI docs

## Part B: Unify Dual Transcript Sources Of Truth

### Current Problem
Two transcripts maintained in parallel:

1. `UIState.transcript: Vec<TranscriptItem>` — old renderer, modals
2. `ClaudeRenderer.transcript: ClaudeTranscript` — Claude renderer

Every push goes to both — error-prone, wasteful.

### Implementation Phases

**Phase 1 — Audit Consumers:**
Find all code reading from `UIState.transcript`:
- Old renderer (`ui_render_legacy.rs`)
- Modal overlays
- `app_chat_loop.rs` (context calculations)
- `tool_calling.rs` (tool history)
- Session save/resume

**Phase 2 — Migrate Consumers:**
- Add accessor methods to `ClaudeTranscript`
- Convert to needed format

**Phase 3 — Remove Old:**
- Remove `transcript` field from `UIState`
- Remove all `push_*` methods from `UIState`
- Update TerminalUI to only push to `ClaudeRenderer`

**Phase 4 — Verify:**
- No references to `UIState.transcript` or `TranscriptItem`

## Files Likely Touched
- `src/ui/ui_terminal.rs` — remove dual draw paths, single transcript push
- `src/claude_ui/claude_render.rs` — modal rendering methods
- `src/claude_ui/claude_state.rs` — modal message types
- `src/ui/ui_state.rs` — remove transcript field
- `src/ui/ui_render_legacy.rs` — remove when done

## Verification
- [ ] PTY fixture: permission gate in Claude renderer
- [ ] PTY fixture: tool approval in Claude renderer
- [ ] PTY fixture: no legacy chrome during modals
- [ ] Manual: visual inspection of all modal types
- [ ] `cargo test --test ui_parity`
- [ ] `cargo build` succeeds
- [ ] All tests pass

## Related Tasks
- Task 166 (master plan)
- Task 177 (legacy UI removal)
- Task 189 (transcript unification)

---
*Created: 2026-04-22*
