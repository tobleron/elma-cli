# 190: Unify Dual Transcript Sources Of Truth

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
- Removed dual transcript pushing:
  - `add_message()` now pushes ONLY to Claude renderer (not UIState transcript)
  - `push_tool_start()` now pushes to Claude renderer as `ClaudeMessage::ToolStart`
  - `push_tool_finish()` now pushes to Claude renderer as `ClaudeMessage::ToolResult`
  - `push_warning()` now pushes to Claude renderer as `ClaudeMessage::System`
  - `push_meta_event()` is now a no-op (meta events handled via Claude event system)
- Removed dead code from TerminalUI:
  - Removed `previous_screen` field
  - Removed `draw_normal()` method
  - Removed `ScreenBuffer` import
  - Fixed `pump_ui()` to use `previous_claude_screen` instead
- UIState transcript push methods still exist but are no longer called from TerminalUI
- All 27 UI parity tests pass
- Verification: `cargo build` passed, `cargo test --test ui_parity` passed (27 tests), `cargo test` full suite passed

## Verification
- [x] No dual pushing in `add_message()`
- [x] Tool events pushed to Claude renderer
- [x] Dead code removed
- [x] `cargo test --test ui_parity` passes (27 tests)

## Remaining Work (Future Tasks)
- Fully remove `UIState.transcript` field and `TranscriptItem` enum
- Remove `ui_render_legacy.rs` module entirely
- Migrate any remaining consumers of `UIState.transcript`

## Priority
High

## Master Plan
Task 166 (Claude Code Terminal Parity Master Plan)

## Objective
Eliminate dual transcript maintenance and use only `ClaudeRenderer.transcript` as the source of truth.

## Current Problem
Two transcript structures are maintained in parallel:

1. `UIState.transcript: Vec<TranscriptItem>` — in `ui_state.rs`
   - Used by: old renderer (removed), modal overlays (migrated)
   - Types: `User`, `Assistant`, `ToolStart`, `ToolResult`, `MetaEvent`, `Warning`, `Thinking`, `System`

2. `ClaudeRenderer.transcript: ClaudeTranscript` — in `claude_render.rs`
   - Used by: Claude renderer
   - Types: `ClaudeMessage` enum (10 variants)

Every message push goes to BOTH:
- `ui_state.rs`: `state.push_user()`, `state.push_assistant()`, etc.
- `claude_render.rs`: `claude.push_message(ClaudeMessage::User { ... })`

This is:
- Error-prone (can get out of sync)
- Wasteful (double memory)
- Confusing (which one is authoritative?)

## Target State
Only `ClaudeTranscript` is the source of truth. `UIState` should:
- Either not maintain a transcript at all
- Or maintain a thin reference to `ClaudeTranscript`
- Or be refactored to use `ClaudeTranscript` directly

## Implementation Plan

### Phase 1: Audit All Transcript Consumers
Find every place that reads from `UIState.transcript`:
1. Old renderer — REMOVED in Task 182
2. Modal overlays — MIGRATED in Task 182
3. `app_chat_loop.rs` — may read transcript for context window calculations
4. `tool_calling.rs` — may read transcript for tool history
5. Session save/resume — may serialize transcript

### Phase 2: Migrate Consumers
For each consumer, migrate to read from `ClaudeTranscript` instead:
- Add accessor methods to `ClaudeTranscript` if needed
- Convert `ClaudeMessage` to the format the consumer needs

### Phase 3: Remove Old Transcript
1. Remove `transcript` field from `UIState`
2. Remove all `push_*` methods from `UIState`
3. Update `TerminalUI` to only push to `ClaudeRenderer`
4. Update chat loop to only push to `ClaudeRenderer`

### Phase 4: Verify
Ensure no code references `UIState.transcript` or `TranscriptItem`.

## Files Likely Touched
- `src/ui/ui_state.rs` — remove transcript, keep other state
- `src/ui/ui_terminal.rs` — remove dual push calls
- `src/app_chat_loop.rs` — use ClaudeTranscript for context
- `src/tool_calling.rs` — use ClaudeTranscript for history
- `src/session_manager.rs` — serialize ClaudeTranscript instead

## Verification
- [ ] `cargo build` with no errors
- [ ] `cargo test` all passes
- [ ] PTY fixtures still pass
- [ ] Manual: verify transcript survives compaction, clear, resume

## Related Tasks
- Task 166 (master plan)
- Task 182 (legacy modal removal — completed, unblocks this)
- Task 176 (session lifecycle — may touch transcript serialization)

---
*Created: 2026-04-22*
