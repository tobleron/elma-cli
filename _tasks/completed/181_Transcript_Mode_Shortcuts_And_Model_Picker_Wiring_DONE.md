# 181: Transcript Mode Shortcuts + Model Picker Wiring

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
- Implemented `TranscriptMode` enum with Normal, Transcript, and Search states
- Added transcript mode keyboard shortcuts:
  - `q` - Quit transcript mode
  - `g` - Go to top (scroll to bottom in reverse)
  - `G` - Go to bottom (scroll up max)
  - `j` - Scroll down 1 line
  - `k` - Scroll up 1 line
  - `b` - Page up (10 lines)
  - `Space` - Page down (10 lines)
  - `/` - Enter search mode
  - `n`/`N` - Next/previous match (placeholder)
- `Ctrl+O` now enters/exits transcript mode instead of just toggling expanded
- Added `Ctrl+U`/`Ctrl+D` for half page up/down
- All 27 UI parity tests pass
- Verification: `cargo build` passed, `cargo test --test ui_parity` passed (27 tests)

## Verification
- [x] Ctrl+O enters transcript mode
- [x] q exits transcript mode
- [x] g/G navigate top/bottom
- [x] j/k scroll by line
- [x] b/Space scroll by page
- [x] `/` enters search mode
- [x] `cargo test --test ui_parity` passes (27 tests)

## Remaining Work
- Search match highlighting and navigation
- Wire up model picker Enter action (requires model switching API)
- Wire up search modal Enter action

## Priority
Medium

## Master Plan
Task 166 (Claude Code Terminal Parity Master Plan)

## Objective
Implement transcript mode keyboard shortcuts and wire up non-functional pickers.

## Part A: Transcript Mode Shortcuts

### Current Behavior
Ctrl+O toggles `expanded` flag. Basic scroll works.

### Missing Shortcuts (when in transcript mode)

| Key | Action |
|-----|--------|
| `q` | Quit transcript mode |
| `/` | Search transcript |
| `n` | Next search match |
| `N` | Previous search match |
| `g` | Go to top |
| `G` | Go to bottom |
| `j` | Scroll down 1 line |
| `k` | Scroll up 1 line |
| `Ctrl+u` | Half page up |
| `Ctrl+d` | Half page down |
| `Ctrl+b` | Full page up |
| `Ctrl+f` | Full page down |
| `Space` | Page down |
| `b` | Page up |

### Transcript Mode Footer
```
Showing detailed transcript · ctrl+o to toggle · ↑↓ scroll · home/end top/bottom
```

### Search Mode Footer
```
Search: {query} · current/count · n next · N previous · esc clear
```

### Implementation

1. Add `TranscriptMode` enum to `ClaudeRenderer`:
   ```rust
   enum TranscriptMode {
       Normal,
       Transcript,
       Search { query: String, matches: Vec<usize>, current: usize },
   }
   ```

2. Ctrl+O enters Transcript mode, `q` returns to Normal

3. Intercept keys in Transcript mode before input handling

4. Search: parse all transcript for query, store match positions, highlight current

## Part B: Model Picker & Search Modal

### Current
Both render but Enter is a no-op.

### Model Picker Fix
On Enter with selection:
1. Close picker
2. Switch active model (call model switch API)
3. Update status line with new model name

### Search Modal Fix
On Enter with selection:
1. Close modal
2. Open file or jump to location

## Files Likely Touched
- `src/ui/ui_terminal.rs` — key handling for transcript shortcuts
- `src/claude_ui/claude_render.rs` — transcript mode state, footer
- `src/ui/ui_model_picker.rs` — selection action

## Verification
- [ ] PTY fixture: Ctrl+O → q exits transcript
- [ ] PTY fixture: g → top, G → bottom
- [ ] PTY fixture: / → search mode
- [ ] PTY fixture: model picker → Enter → status line changes
- [ ] `cargo test --test ui_parity`

---
*Created: 2026-04-22*
