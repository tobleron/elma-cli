# Ratatui UI Enhancement Suggestions

> Generated from deep analysis of opencrabs ratatui patterns vs current Elma renderer.
> Date: 2026-04-28

---

## Deep Analysis Summary

### Sources Analyzed

| Source | What was examined |
|--------|------------------|
| `src/claude_ui/claude_render.rs` | Full render pipeline: layout (l696-711), footer (l1003-1028, l1135-1236), input (l875-902), background (l668-671) |
| `src/ui/ui_theme.rs` | Theme tokens: ColorToken, Theme struct, default_theme colors, fg/fg_dim/accent_* |
| `src/ui/ui_terminal.rs` | Sync loop: context calc (l816-828), input sync (l800-802), footer model set (l842-847) |
| `_knowledge_base/_source_code_agents/opencrabs/src/tui/render/mod.rs` | Master render layout, dynamic input height, Constraint::Min |
| `_knowledge_base/_source_code_agents/opencrabs/src/tui/render/input.rs` | Input wrapping via `wrap_line_with_padding`, cursor rendering, context-color-coded titles |
| `_knowledge_base/_source_code_agents/opencrabs/src/tui/render/status_bar.rs` | Status bar: provider/model on left, policy center-right, multiple spans per line |
| `_knowledge_base/_source_code_agents/opencrabs/src/tui/render/utils.rs` | Character-aware wrapping, boundary-safe truncation, token count formatting |
| `_knowledge_base/_source_code_agents/opencrabs/src/tui/render/panes.rs` | Border/padding patterns, Block::inner() for margin enforcement |
| `_knowledge_base/_source_code_agents/opencrabs/src/tui/app/state.rs` | Context tracking: real token counts, context_max_tokens, percentile method |

### Key Findings

**1. Footer background is pure black, not dark grey**

AGENTS.md specifies "Dark grey" for background, but `claude_render.rs:669` uses `Color::Black` for the full-frame background block. The footer gets a black background instead of the intended dark grey.

*OpenCrabs* uses the terminal's default background (no explicit background block) and reserves colored backgrounds only for active states (inverted cursor, selected items).

**2. No side margins — content touches screen edges**

Elma's layout (`claude_render.rs:696-705`) splits `area` with zero horizontal margins. The transcript, input, and footer all span the full terminal width edge-to-edge.

*OpenCrabs* uses `Block::padding()` and explicit border/margin areas. The input box has `Borders::TOP | BOTTOM` with a known content width of `area.width.saturating_sub(2)`. Panes use `Padding::horizontal(1)`.

**3. Input does not wrap to terminal width**

`ui_terminal.rs:800` syncs input as `self.input.lines().to_vec()` — split only by user-inserted newlines. Long input lines overflow past the screen edge instead of wrapping.

*OpenCrabs* calculates `input_content_width = area.width.saturating_sub(2)` and wraps every line using `wrap_line_with_padding()`, which:
- Measures display width via `UnicodeWidthStr::width()`
- Finds word breaks where possible
- Prepends continuation padding ("  ") to wrapped lines

**4. Footer layout is left-aligned only — no right-side content**

`render_footer_line()` (l1159-1228) concatenates all segments left-to-right with `"  "` separators. Context, model name, transcript metric, and streaming state are all crammed left. There's no way to put the model name far right.

*OpenCrabs* status bar (`render_status_bar`):
- Left: session name (orange, bold) + provider/model/directory (bluish)
- Right (via border title): context percentage with color coding
- Dynamic padding between left and right elements

**5. Context calculation is a rough character-count estimate**

`ui_terminal.rs:816-828` estimates context usage via:
```rust
let streaming_tokens = (thinking.len() + content.len()) / 4;  // chars÷4 ≈ tokens
let model_context_tokens_estimate = context_current + streaming_tokens;
let ctx_pct = (model_context_tokens_estimate * 100) / context_max;
```

*OpenCrabs* uses actual token counts from the API response (`token_count` field in each message) tracked per-session via `last_input_tokens: Option<u32>`, excluding tool schema overhead.

**6. No color-coded context or status indicators**

Elma renders all footer segments with `theme.fg_dim` (grey 128,128,128). There's no visual distinction between low/high context usage.

*OpenCrabs* color-codes context percentage:
- ≤60%: Cyan
- 60-80%: Orange `Color::Rgb(215, 100, 20)`
- >80%: Red
- Uses `Modifier::BOLD` for emphasis

---

## Enhancement Tasks

Each task is self-contained and can be implemented independently.
Reject any task you don't want — the rest remain valid.

### Status Legend
- ✅ Completed
- ⬜ Not started
- 🚫 Rejected (by user)

---

### TASK-001: Footer background color — black to dark grey ✅

**Problem**: Background block at `claude_render.rs:668-671` uses `Color::Black` instead of the dark grey specified in AGENTS.md.

**Change**: Add a `bg` token to `Theme` (dark grey ~28,28,30) and a `bg_footer` token (slightly lighter ~36,36,40) and use them in the background block. Also use them for footer line via `Paragraph::style()`.

**Files**: `src/ui/ui_theme.rs` (add bg/bg_footer tokens), `src/claude_ui/claude_render.rs` (use new bg tokens for background block, footer, and prompt_hint)

**Difficulty**: Low | **Risk**: Low

---

### TASK-002: Add horizontal gutter/margin to all content areas ✅

**Problem**: Transcript, input, and footer render edge-to-edge with no margins.

**Change**: Apply a 1-column horizontal margin by creating a `content_area` with `area.x + gutter`, `area.width.saturating_sub(gutter * 2)`. All layout splits use `content_area` instead of `area`.

**Files**: `src/claude_ui/claude_render.rs` (layout block wrapping or margin-aware splits)

**Difficulty**: Low | **Risk**: Medium (must adjust all area calculations)

---

### TASK-003: Wrap input lines to terminal width ✅

**Problem**: Long input lines overflow past the screen edge instead of wrapping. `input.lines()` returns user-inserted newlines only.

**Change**: Before rendering input, measure each line's display width (via `unicode_width::UnicodeWidthChar`) and split into wrapped lines. Use `wrap_input_lines()` to produce continuation lines with proper indentation. Dynamic input height based on wrapped count capped at 10 lines. Cursor position recalculated for wrapped lines via `cursor_in_wrapped()`.

**Files**: `src/claude_ui/claude_render.rs` (input rendering + new helper functions `wrap_input_lines`, `cursor_in_wrapped`, `char_display_width`, `str_display_width`)

**Difficulty**: Medium | **Risk**: Medium (cursor positioning must be recalculated for wrapped lines)

---

### TASK-004: Move model name to far right of footer ✅

**Problem**: All footer segments are left-aligned with `"  "` separators. Model name should be on the far right.

**Change**: Restructured `render_footer_line` to use three sections: left (mode_label in pink+bold), center (context bar + streaming state in gray), right (model name in gray). Uses Span padding for distribution. Footer background uses `bg_footer` token.

**Files**: `src/claude_ui/claude_render.rs` (render_footer_line function)

**Difficulty**: Low | **Risk**: Low

---

### TASK-005: Accurate context calculation using token counts ⬜

**Problem**: Context is estimated via `chars/4` which is inaccurate (3-5x off for code with lots of special chars). Real token counts from API responses should be used.

**Change**: Track per-message token counts from model responses. Pass actual token counts to `FooterModel`. Remove the character-estimate fallback.

**Files**: `src/ui/ui_terminal.rs` (context calc l816-828), `src/claude_ui/claude_state.rs` or wherever footer model is populated

**Difficulty**: Medium | **Risk**: Low (purely additive — estimate remains as fallback)

---

### TASK-006: Color-code context percentage in footer 🚫

**Problem**: All footer segments use the same grey color regardless of context pressure.

**User rejected**: Footer should stay gray. Mode labels should be in primary color (pink).

**Files**: N/A — rejected

**Difficulty**: N/A | **Risk**: N/A

---

### TASK-007: Mode label in primary color on footer ✅

**User's requirement**: Footer is stuck at bottom, gray, with modes (plan/normal/skill) displayed in primary color (pink, bold). Model name on far right.

**Change**: Added `mode_label: Option<String>` to `FooterModel`. In `render_footer_line`, mode label rendered in `accent_primary` + `BOLD`. Derived from `self.state.header.workflow` field (empty = no label, non-empty = mode name like "plan", "skill", etc.).

**Files**: `src/claude_ui/claude_render.rs` (FooterModel, render_footer_line), `src/ui/ui_terminal.rs` (FooterModel construction with mode_label)

**Difficulty**: Low | **Risk**: Low

---

### TASK-008: Splash rendering at top of transcript ✅

**User's requirement**: Smaller pink ELMA ASCII splash at top of transcript that scrolls with content.

**Change**: Added `render_splash()` function that renders 6-line ASCII ELMA in primary color (pink) + "local-first AI assistant" subtitle in dim. Injected at render time when `splash_active` is true (first render only). Splash scrolls naturally as transcript content grows.

**Files**: `src/claude_ui/claude_render.rs` (render_splash function, injection in render_ratatui)

**Difficulty**: Low | **Risk**: Low

---

### TASK-009: Slash command Tab autocomplete ✅

**User's requirement**: When `/` picker is active, Tab should auto-complete the selected slash command (replacing the input text), and Enter should execute the command immediately.

**Change**: Added `KeyCode::Tab` handling in the picker-active key handler in `ui_terminal.rs`. Tab calls `selected_slash_command()` and sets the input content to the full command string, re-opening the picker with the new query for further refinement. Enter already executed commands immediately (no change needed).

**Files**: `src/ui/ui_terminal.rs` (Tab handler in picker-active block)

**Difficulty**: Low | **Risk**: Low

---

### TASK-010: Footer visual weight with distinct background ✅

**Problem**: Footer had no visual distinction from content area.

**Change**: Footer area uses `bg_footer` color token (36,36,40) — slightly lighter than the main background (28,28,30). All footer spans (prompt hints, default hints, footer line) carry the `bg_footer` background style.

**Files**: `src/claude_ui/claude_render.rs`, `src/ui/ui_theme.rs`

**Difficulty**: Low | **Risk**: Low

---

### TASK-011: Per-message token counting to transcript ⬜

**Problem**: The transcript metric in the footer (`tx NNN`) only shows when verbose and raw token estimate exceeds context. It's not useful for normal use.

**Change**: Track per-message token counts (from API responses) and display meaningful token usage alongside context percentage. Show transcript total tokens in footer as `ctx 45% (12K/26K)`.

**Files**: `src/ui/ui_terminal.rs`, `src/claude_ui/claude_state.rs`

**Difficulty**: Medium | **Risk**: Low

---

### Task Dependencies

```
TASK-001 (bg color) ── ✅ completed
TASK-002 (margins)  ── ✅ completed
TASK-003 (wrap)     ── ✅ completed
TASK-004 (model right) ── ✅ completed (merged into TASK-007)
TASK-005 (token calc) ── ⬜ not started
TASK-006 (color pct) ── 🚫 rejected by user
TASK-007 (mode in primary) ── ✅ completed
TASK-008 (splash) ── ✅ completed
TASK-009 (tab autocomplete) ── ✅ completed
TASK-010 (footer bg) ── ✅ completed (merged into TASK-001)
TASK-011 (transcript tokens) ── ⬜ not started
```

---

## Summary of OpenCrabs Patterns Worth Adopting

| Pattern | OpenCrabs Implementation | Elma Current State |
|---------|-------------------------|-------------------|
| Dynamic input height | `input_height = min(wrapped_lines + 2, 10)` | `Constraint::Length(lines.len())` — no wrapping |
| Input wraps to width | `wrap_line_with_padding(line, max_width, "  ")` | Raw line strings, overflow |
| Status bar multi-section | Left: model/provider/dir, Right: ctx% | Left-only flat concatenation |
| Context color coding | Cyan ≤60%, Orange 60-80%, Red >80% | Always grey |
| Real token counting | `last_input_tokens` from API + per-message counts | `chars/4` estimate |
| Horizontal padding | `Padding::horizontal(1)` | None |
| Block-based borders | `Block::default().borders(Borders::TOP \| BOTTOM)` | None on input |
| Cursor on character | Inverse highlight on the character at cursor pos | Not rendered (cursor managed externally) |
| Emoji/slash pickers | Popup above input with bordered Block | Inline in transcript area |
