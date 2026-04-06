# Task 133: Implement Ratatui TUI with Gruvbox Colors and Persistent Status Bar

## Priority
**P0 — Visual Reliability**
**Created:** 2026-04-06
**Status:** Pending
**Dependencies:** Task 132 (Claude Code UI design reference)

## Problem

The current Elma CLI uses `println!` for all output. There is no real TUI. The `ui_tui.rs` module contains a `TerminalUI` struct with ratatui rendering code but it is **NOT wired into the application**. The chat loop still uses `prompt_line()` (raw stdin) and `print_elma_message()` (println). The status bar scrolls away immediately. There is no persistent visual structure.

## Target Design Reference

Study `_stress_testing/_claude_code_src/` for the visual layout reference. The key patterns to replicate:

### 1. Message Row Structure (from `components/Message.tsx`)
```
●  [assistant markdown content...]

  ● shell (ls -la)
  ✓ execution complete

> [user input here]
```

- `●` (U+25CF BLACK CIRCLE) as message prefix dot — colored
- Assistant messages: dot + rendered markdown (no response-level borders)
- Tool messages: dot + **TOOL_NAME** in bold badge + (details in parens)
- Tool results: `✓` (green) or `✗` (red) status
- User input: `>` prompt with yellow prefix

### 2. Markdown Rendering (from `components/Markdown.tsx`)
- `# H1` → bold + italic + underline
- `## H2+` → bold
- `**bold**` → bold (white text)
- `_italic_` / `*italic*` → italic (dim color)
- `` `code` `` → accent color (mauve in current palette, but use Gruvbox Orange)
- `> quote` → `▎` (U+258E) prefix + italic (dim color)
- `- item` → `•` (U+2022) bullet + text
- `1. item` → numbered list
- `---` → `─` (U+2500) horizontal rule spanning width
- Code fences → syntax highlighted via syntect, **NO borders** around code

### 3. Tool Use Display (from `components/AssistantToolUseMessage.tsx`)
- **In progress**: `● TOOL_NAME (command)` with dim dot
- **Success**: `✓ TOOL_NAME — completed` with green check
- **Error**: `✗ TOOL_NAME — failed` with red cross
- Tool name is **BOLD** with accent color
- Command/details in dim color inside parens
- Tool output rendered directly below (full, never truncated unless >100 lines → show first 50 + "N more lines in session logs")

### 4. Prompt Input (from `components/PromptInput/PromptInput.tsx`)
- Clean `>` prefix in yellow/accent color
- Text input with backspace support
- Enter to submit, Esc to cancel
- No input box chrome — just the prompt line

## Color Palette: Gruvbox Dark Hard (Extra Contrast)

**Only use these colors.** No Tokyo Night. No Catppuccin. No Rose Pine.

| Color Name | Hex | RGB | Semantic Usage |
|------------|-----|-----|---------------|
| **Bg Hard** | `#1d2021` | `29, 32, 33` | Background (terminal provides this) |
| **Fg** | `#ebdbb2` | `235, 219, 178` | Primary text, normal output |
| **Red** | `#fb4934` | `251, 73, 52` | Errors, failures, destructive blocks |
| **Green** | `#b8bb26` | `184, 187, 38` | Success, confirmations, safe operations |
| **Yellow** | `#fabd2f` | `250, 189, 47` | Warnings, caution commands, prompts, tool names |
| **Blue** | `#83a598` | `131, 165, 152` | Tool execution info, informational messages |
| **Purple** | `#d3869b` | `211, 134, 155` | Accent, inline code, Elma prefix |
| **Aqua** | `#8ec07c` | `142, 192, 124` | Secondary accent, tool badges |
| **Orange** | `#fe8019` | `254, 128, 25` | Highlights, warnings, important markers |
| **Gray** | `#928374` | `146, 131, 116` | Metadata, timestamps, dim text (Overlay0) |

**Design rationale:** Gruvbox Dark Hard has the highest contrast of any major terminal theme. The warm tones are easy on the eyes during long sessions. The yellow/red/green are vivid and unambiguous. Purple is the distinctive accent for Elma's brand.

## Architecture: Full Ratatui Alternate Screen TUI

### Current State
```
src/ui_tui.rs          ← TerminalUI struct exists but UNUSED
src/ui_trace.rs        ← print_elma_message uses println! (OLD PATH)
src/app_chat_loop.rs   ← prompt_line() uses raw stdin (OLD PATH)
```

### Target State
```
src/ui_tui.rs          ← Primary TUI: ratatui alternate screen
src/app_chat_loop.rs   ← Uses TerminalUI for input/output
src/app_chat_helpers.rs ← Updates status bar data on tool events
```

### Required Changes

#### 1. `src/ui_colors.rs` — Replace palette
- Replace ALL Catppuccin/Tokyo Night/Rose Pine colors with Gruvbox
- Update all function outputs: `elma_accent`, `info_cyan`, `error_red`, `warn_yellow`, `success_green`, `text_white`, `meta_comment`
- Update ratatui color mappings

#### 2. `src/ui_tui.rs` — Complete ratatui TUI
The existing `TerminalUI` struct is a good starting point. Enhance it:

**Structure:**
```
┌─────────────────────────────────────────┐
│                                         │
│  Content Area (messages)                │
│  ● Assistant response with markdown     │
│    # Header                             │
│    **bold** and `code`                  │
│                                         │
│    ```rust                              │
│    fn main() { }                        │
│    ```                                  │
│                                         │
│  ● shell (ls -la)                       │
│  ✓ execution complete                   │
│    file1  file2  file3                  │
│                                         │
│  > user typing here...                  │
│                                         │
├─────────────────────────────────────────┤
│ granite-4.0 · 38% context · ⏱ 2.3s     │ ← Persistent status bar (bottom 1 row)
└─────────────────────────────────────────┘
```

**Required components:**
- `render_screen()` — Layout: content area (min) + status bar (1 row)
- `render_messages()` — Scroll through messages, render markdown
- `render_status_bar()` — Model · context% · effort time
- `render_md_line()` — Parse and render individual markdown lines
- `parse_inline_md()` — Handle `**bold**`, `*italic*`, `` `code` ``
- Input handling: Char, Backspace, Enter, Esc
- `cleanup()` — LeaveAlternateScreen + disable raw mode

**Terminal lifecycle:**
```rust
// App bootstrap:
let mut tui = TerminalUI::new()?;
tui.update_status(model_id, 0, ctx_max, String::new());

// Chat loop:
loop {
    let input = tui.run_input_loop()?;
    tui.add_message(MessageRole::User, input.clone());
    // ... process through intel/tools ...
    tui.add_message(MessageRole::Assistant, response);
    tui.update_status(model, current_tokens, max_tokens, effort);
}

// Shutdown:
tui.cleanup()?;
```

#### 3. `src/app_chat_loop.rs` — Replace I/O layer
**Remove:**
- `prompt_line()` calls → use `tui.run_input_loop()`
- Direct `println!` for system messages → use `tui.add_message()`

**Replace:**
```rust
// OLD:
let Some(line) = prompt_line(&prompt)? else { break; };

// NEW:
let Some(line) = tui.run_input_loop()? else { break; };
```

#### 4. `src/app_chat_helpers.rs` — Status bar integration
**Replace** the current `print_final_output` footer with:
```rust
tui.update_status(
    model_id.clone(),
    final_usage_total.unwrap_or(0),
    ctx_max.unwrap_or(0),
    effort_timer.format(),
);
```

#### 5. `src/tool_calling.rs` — Tool display via TUI
When a tool executes:
```rust
// Before execution:
tui.add_message(MessageRole::Tool {
    name: "shell".to_string(),
    command: command.clone(),
}, String::new());

// After execution:
tui.add_message(MessageRole::ToolResult { success: er.exit_code == 0 }, String::new());
```

#### 6. `src/ui_trace.rs` — Remove old print path
**Remove** `print_elma_message()` — replace with TUI message adding:
```rust
// OLD:
println!("{}", crate::ui_layout::render_assistant_message(&rendered));

// NEW (handled through tui.add_message):
// Nothing — the TUI renders it
```

### Special Characters (Unicode)
| Usage | Character | Unicode | Color |
|-------|-----------|---------|-------|
| Message dot | `●` | U+25CF | Accent (Purple → Gruvbox Purple) |
| Success check | `✓` | U+2713 | Green |
| Error cross | `✗` | U+2717 | Red |
| Blockquote bar | `▎` | U+258E | Gray (dim) |
| List bullet | `•` | U+2022 | Blue |
| Prompt prefix | `>` | ASCII 62 | Yellow |
| Horizontal rule | `─` | U+2500 | Gray (dim) |
| Separator | `·` | U+00B7 | Gray (dim) |

### Status Bar Content

The persistent status bar (bottom row, dim color) must show:

```
{model_id}  ·  {context_pct}% context  ·  {token_in}/{token_out} tokens  ·  {effort_time}
```

Example:
```
granite-4.0-h-micro  ·  38.3% context  ·  47.1k/12.4k tokens  ·  ⏱ 2.3s
```

- **Model**: Current model ID (shortened if too long)
- **Context %**: Current usage / context window size, color-coded:
  - < 70%: Green
  - 70-90%: Yellow
  - > 90%: Red
- **Tokens**: Input tokens / Output tokens for current session
- **Effort**: Wall-clock time for current turn (⏱ prefix)

### Markdown Rendering Requirements

All markdown features must be rendered in-terminal:

| Markdown | Rendering | Color |
|----------|-----------|-------|
| `# H1` | `# text` bold + italic + underline | Yellow |
| `## H2` | `## text` bold | Yellow |
| `### H3` | `  text` bold | Yellow |
| `**bold**` | **bold** | Fg (white) |
| `_italic_` | *italic* | Gray (dim) |
| `` `code` `` | `code` | Purple (accent) |
| `> quote` | `▎ italic text` | Gray (dim) |
| `- item` | `• item` | Blue bullet, Fg text |
| `1. item` | `1. item` | Yellow number, Fg text |
| `---` | `─` × width | Gray (dim) |
| `~~~lang ... ~~~` | Syntax highlighted code | Per-language |

**Code blocks:**
- Use `ui_syntax.rs` (syntect) for syntax highlighting
- NO borders around code blocks (Claude Code style)
- Language label shown above code (dim color)
- Full content shown — never truncated in TUI

### Design Principles

1. **Minimalistic**: No decorative borders around responses. Let markdown formatting create structure.
2. **Content-first**: The response is the content, not the chrome around it.
3. **No truncation**: Full output always visible. Very long tool outputs (>100 lines) show first 50 with "N more lines" note.
4. **Persistent status bar**: Bottom row never scrolls away. Always shows model, context, tokens, time.
5. **Gruvbox only**: No other color palette. Every color maps to the Gruvbox palette above.
6. **Professional but friendly**: Clean layout, subtle animations (spinner for in-progress tools), clear status indicators.
7. **Claude Code parity**: Match the visual quality and structure of `_stress_testing/_claude_code_src/`.

### Technical Constraints

- **Must use ratatui** for the alternate screen TUI (already in dependencies)
- **Must use crossterm** for terminal control (already in dependencies)
- **Must use syntect** for syntax highlighting (already in dependencies)
- **No new dependencies** beyond what's in `Cargo.toml`
- **Must handle terminal resize** gracefully (ratatui handles this)
- **Must clean up on exit** (LeaveAlternateScreen + disable raw mode)
- **Must support Ctrl+C** for graceful shutdown
- **Must work in non-interactive mode** (piped input) — fall back to println

### Verification

1. `cargo build` — zero warnings
2. `cargo test` — all green
3. Real CLI: Start Elma → see alternate screen TUI
4. Real CLI: Type message → see `>` prompt with Gruvbox yellow
5. Real CLI: Assistant responds → see `●` (purple) + rendered markdown
6. Real CLI: Tool executes → see `shell (command)` with yellow tool name
7. Real CLI: Status bar → see model · context% · tokens · time at bottom, never scrolls
8. Real CLI: Press Esc → exit alternate screen, return to normal terminal
9. Real CLI: Pipe input → falls back to println mode gracefully

### Acceptance Criteria
- [ ] `src/ui_colors.rs` uses ONLY Gruvbox Dark Hard colors
- [ ] `src/ui_tui.rs` renders full ratatui alternate screen TUI
- [ ] Persistent status bar at bottom with model, context%, tokens, effort
- [ ] Assistant messages render with `●` prefix + full markdown
- [ ] Tool execution shows `TOOL_NAME` badge + command + output
- [ ] Status bar color-codes context usage (green/yellow/red)
- [ ] No heavy borders around responses (Claude Code style)
- [ ] No truncation of any output
- [ ] Gruvbox colors throughout (purple accent, yellow warnings, green success, red errors, blue info, gray dim)
- [ ] Clean exit restores normal terminal (no broken state)
- [ ] Piped/non-interactive input falls back to println

### Files to Modify
- `src/ui_colors.rs` — Gruvbox palette
- `src/ui_tui.rs` — Full ratatui TUI (enhance existing struct)
- `src/ui_markdown.rs` — Markdown rendering (align with Gruvbox)
- `src/ui_layout.rs` — Simplify (remove old border code)
- `src/app_chat_loop.rs` — Replace stdin/stdout with TUI
- `src/app_chat_helpers.rs` — Status bar integration
- `src/tool_calling.rs` — Tool display via TUI
- `src/ui_trace.rs` — Remove old print_elma_message

### Reference Files (existing, good starting points)
- `src/ui_tui.rs` — Has TerminalUI struct, needs wiring
- `_stress_testing/_claude_code_src/components/Message.tsx` — Layout reference
- `_stress_testing/_claude_code_src/components/Markdown.tsx` — Markdown rendering reference
- `_stress_testing/_claude_code_src/components/StatusLine.tsx` — Status bar reference

## Notes
- The `ui_tui.rs` file already has a `TerminalUI` struct with `draw()`, `render_screen()`, etc. but it's not wired into the app. Use it as the foundation.
- The `app_chat_loop.rs` currently uses `prompt_line()` (raw stdin) and `print_final_output()` (println). These need to be replaced with TUI calls.
- Do NOT break the existing tool execution, safety checks, or intel pipeline. Only change the display layer.
- The status bar must be truly persistent — not just printed at the end and scrolled away. It must be rendered at the bottom of the screen on every draw.
