# Task 311: Persistent Status Thread — Always-Visible Activity Indicator

**Status**: Pending  
**Priority**: High  
**Depends on**: None (additive, no breaking changes)  
**Elma Philosophy**: Principle-first, small-model-friendly, offline-first

## Problem

Elma currently uses `clear_activity()` to hide the activity indicator once processing completes. The user has no persistent visibility into what Elma is doing — especially background processes like the turn summarizer. When Elma is thinking or running tools, the activity indicator disappears too quickly for the user to read what happened.

## Solution: Persistent Status Thread

A status thread that:
1. **Always stays visible** at the very end of the chat history (above the input prompt)
2. **Shows a spinner animation** (⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏) to indicate active work
3. **Displays the current operation** ("Summarizing previous turn...", "Reading Cargo.toml...", "Thinking...")
4. **Persists for minimum 2 seconds** after completion so the user has time to see what happened
5. **Transitions through states**: active → completed (✓) → faded → hidden

## Architecture

```
┌─────────────────────────────────────────┐
│ User: Fix the build errors              │
│                                         │
│ Elma: I found 3 issues...               │
│   • Missing import in lib.rs            │
│   • Type mismatch in main.rs            │
│                                         │
│ ─────────────────────────────────────── │
│ ⠋ Summarizing previous turn...          │  ← STATUS THREAD (always visible)
│                                         │
│ > _                                     │
└─────────────────────────────────────────┘
```

### Status States

```rust
pub(crate) enum StatusState {
    Idle,
    Working { description: String, started_at: Instant },
    Completed { description: String, completed_at: Instant },
}
```

### Spinner Animation

Braille spinner: `⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧ ⠇ ⠏` — advances every 100ms on each render cycle.

### Minimum 2-Second Visibility Rule

The `min_visible_until` field ensures that even if `clear()` is called immediately after `complete()`, the status line remains visible for at least 2 seconds.

```
Timeline:
  0.0s  ┃ ⠋ Reading Cargo.toml...
  0.3s  ┃ ✓ Reading Cargo.toml     ← completed
  0.5s  ┃ ✓ Reading Cargo.toml     ← clear() called, but min_visible_until = 2.5s
  2.0s  ┃ ✓ Reading Cargo.toml     ← still visible
  2.5s  ┃                          ← finally hidden
```

## Implementation Plan

### Part 1: Status Thread Module (`src/ui/ui_status_thread.rs`)
- `StatusThread` struct with state machine, spinner animation, 2s minimum visibility
- Methods: `start()`, `complete()`, `clear()`, `render()`
- Spinner: 10-frame braille sequence, advances every 100ms

### Part 2: UIState Integration (`src/ui/ui_state.rs`)
- Add `status_thread: StatusThread` field to `UIState`
- Add helper methods: `start_status()`, `complete_status()`, `clear_status()`

### Part 3: TerminalUI Integration (`src/ui/ui_terminal.rs`)
- Add `start_status()`, `complete_status()`, `clear_status()` methods that delegate to `UIState`
- Set `pending_draw = true` on status changes to trigger repaint

### Part 4: Claude Renderer Integration (`src/claude_ui/claude_render.rs`)
- In `render_ratatui()`, render status thread as last transcript item (above input)
- Use dimmed color for completed state, normal for working state

### Part 5: Chat Loop Integration (`src/app_chat_loop.rs`)
- Hook into lifecycle points:

| Point | Status |
|-------|--------|
| User submits input | `start_status("Thinking...")` |
| Route/intel analysis | `start_status("Analyzing request...")` |
| Tool execution starts | `start_status("Running {tool}...")` |
| Tool execution ends | `complete_status("{tool} done")` |
| Response displayed | `complete_status("Response ready")` |
| Summarizer starts | `start_status("Summarizing previous turn...")` |
| Summarizer ends | `complete_status("Turn summary saved")` |

## Files to Create/Modify

| File | Action |
|------|--------|
| `src/ui/ui_status_thread.rs` | CREATE |
| `src/ui/ui_state.rs` | MODIFY |
| `src/ui/ui_terminal.rs` | MODIFY |
| `src/claude_ui/claude_render.rs` | MODIFY |
| `src/app_chat_loop.rs` | MODIFY |

## Verification

1. `cargo build` must pass
2. `cargo test` must pass
3. Visual test: run Elma, verify status thread is always visible at bottom of transcript
4. Timing test: trigger fast operation, verify status stays visible for 2s minimum
5. Spinner test: verify spinner animates smoothly during active work
6. Background task test: start summarizer, verify "Summarizing previous turn..." appears
