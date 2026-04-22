# Claude Code Terminal Parity Gap Analysis Plan

## Executive Summary

Based on deep analysis of both the Claude Code source (`_stress_testing/_claude_code_src/`) and the current elma implementation (`src/claude_ui/`, `src/ui/`), here is every remaining gap that needs implementation to achieve full Claude Code parity.

---

## Critical Gaps (P0 — Blocking Task 166 Sign-Off)

### 1. T180: Fix Prompt Non-Editable During In-Flight States
**Status:** EXPLICIT BLOCKER in Task 166 master plan.
**Symptom:** Prompt becomes partially non-editable during model streaming (thinking/content) and tool execution.
**Root Causes:**
- Event loop may block on `crossterm::event::read()` while async work is pending
- `poll_busy_submission()` path has different input handling than idle state
- Possible race conditions between async callbacks and the input loop
**Fix:** Audit event loop for blocking, add non-blocking poll with timeout, protect renderer state.

### 2. 186: Eliminate Legacy Modal Overlay Renderer
**Status:** Causes visual context switch whenever any modal appears.
**Current:** `draw_with_modal()` routes to `ui_render_legacy::render_screen()` with old five-frame layout.
**Target:** All modals render through Claude renderer as absolute-positioned bottom panes with `▔` top border.
**Modals to migrate:** ToolApproval, PermissionGate, PlanProgress, SessionResume, Help, etc.

### 3. 190: Unify Dual Transcript Sources Of Truth
**Status:** Two transcripts maintained in parallel — error-prone and wasteful.
**Current:** `UIState.transcript` (old) + `ClaudeRenderer.transcript` (new)
**Target:** Only `ClaudeTranscript` exists. Remove `UIState.transcript` entirely.
**Blocked by:** Task 186 (legacy modals consume old transcript)

---

## High Priority Gaps (P1 — Visible Parity Issues)

### 4. 180: Sticky Prompt Header When Scrolled Up
**Claude Behavior:** When scrolled up, shows `❯ {truncated prompt}` at top of scrollable area with `userMessageBackground` color.
**Current:** No sticky header. User forgets what prompt they're viewing.
**Files:** `claude_render.rs`, `claude_state.rs`

### 5. 181: New Messages Pill And Unseen Divider
**Claude Behavior:**
- When scrolled up + new messages arrive: `─── N new messages ───` divider in transcript
- Floating pill at bottom: `N new messages ▼` or `Jump to bottom ▼`
- Clicking pill scrolls to bottom
**Current:** No divider, no pill. New messages just appear off-screen.
**Files:** `claude_render.rs`, `claude_state.rs`, `ui_terminal.rs`

### 6. 182: Fix Message Row Indicators And Spacing
**Changes needed:**
- User prefix: `>` → `❯` (`figures.pointer`)
- User truncation: Hard-cap at 10,000 chars (head 2,500 + `… +N lines …` + tail 2,500)
- Assistant spacing: Add `marginTop={1}` blank line before assistant messages
- Thinking transcript: Only show LAST thinking block in normal mode; hide earlier ones
**Files:** `claude_state.rs`, `claude_render.rs`, `ui_theme.rs`

---

## Medium Priority Gaps (P2 — Behavioral Parity)

### 7. 183: Fix Task List Symbols And Add Header
**Symbol changes:**
- Pending: `○` → `◻`
- In-progress: `◐` → `◼` (in `claude` color)
- Completed: `✓` → `✔` (with `strikethrough`)
- Add Blocked status: `▸ blocked by #id`
**Add header:** `N tasks (M done, K in progress, P open)`
**Add truncation:** `… +N in progress, M pending, K completed`
**Files:** `claude_tasks.rs`, `claude_render.rs`

### 8. 184: Transcript Mode Keyboard Shortcuts
**Missing shortcuts when in transcript mode (Ctrl+O):**
| Key | Action |
|-----|--------|
| `q` | Quit transcript mode |
| `/` | Search transcript |
| `n` / `N` | Next/previous search match |
| `g` | Go to top |
| `G` | Go to bottom |
| `j` / `k` | Scroll 1 line |
| `Ctrl+u` / `Ctrl+d` | Half page up/down |
| `Ctrl+b` / `Ctrl+f` | Full page up/down |
**Files:** `ui_terminal.rs`, `claude_render.rs`

### 9. 185: Wire Up Model Picker And Search Modal Enter Actions
**Current:** Both render visually but Enter is a no-op (`// TODO: Switch to selected model`)
**Fix:** Actually switch model / open file when Enter pressed.
**Files:** `ui_terminal.rs`, `ui_model_picker.rs`, `ui_modal_search.rs`

### 10. 187: Shift+Enter For Multiline Input
**Current:** `InputMode::Multiline` exists structurally but no key binding.
**Target:** `Shift+Enter` inserts newline at cursor. Input area grows.
**Files:** `ui_terminal.rs`, `ui_input.rs`

---

## Lower Priority Gaps (P3 — Enhancement)

### 11. 188: Recursive File Picker Workspace Discovery
**Current:** `@` picker only scans top-level directory.
**Target:** Recursive discovery respecting `.gitignore`, skipping `target/`, `node_modules/`, `.git/`.
**Files:** `claude_render.rs`

### 12. 189: Deep Markdown Renderer
**Current:** Code blocks explicitly skipped. No links, tables, numbered lists.
**Target:** `pulldown-cmark` + `syntect` for full markdown with syntax highlighting.
**Files:** `claude_markdown.rs`, `Cargo.toml`

---

## Already Implemented (Verified Working)

- ✅ Left gutter column with status indicators
- ✅ Thinking hidden by default (`false` default)
- ✅ Scrolling with PageUp/Down, arrow keys
- ✅ Context bar: `model-name 67% [█████░░░]`
- ✅ Footer hints
- ✅ `/` slash picker, `@` file picker, `!` bash mode
- ✅ Ctrl+O toggle transcript, Ctrl+T toggle tasks
- ✅ Double Esc clear, Double Ctrl-C/Ctrl-D exit
- ✅ Task list with status symbols (basic)
- ✅ 25 PTY integration tests passing
- ✅ Notification TTL (5-second expiry)
- ✅ Compact boundary hidden by default

---

## Recommended Implementation Order

1. **T180** — Fix prompt lockup (unblocks everything)
2. **186** — Eliminate legacy modal renderer (unblocks 190)
3. **190** — Unify transcript sources (cleanup)
4. **182** — Fix message indicators (high visual impact)
5. **180** — Sticky prompt header (high usability)
6. **181** — New messages pill (high usability)
7. **183** — Task list symbols (medium visual)
8. **184** — Transcript shortcuts (medium usability)
9. **185** — Wire pickers (medium functional)
10. **187** — Shift+Enter (medium functional)
11. **188** — Recursive picker (low)
12. **189** — Deep markdown (low)

---

*Plan generated: 2026-04-22*
*Based on: Claude Code source audit + elma implementation inventory*
