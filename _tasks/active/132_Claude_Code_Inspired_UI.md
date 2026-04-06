# Task 132: Implement Claude Code-Inspired UI

## Priority
**P1 — Visual Reliability**

## Problem
The current Elma CLI has floating text output with no structure. The markdown is not rendered in-terminal, and the visual design does not match modern CLI agent expectations. The Claude Code TUI provides a clean, structured reference for what a premium terminal UI should look like.

## Reference Architecture
Source: `_stress_testing/_claude_code_src/`

Key design patterns studied:
1. **Message Row**: `●` (black circle) prefix + markdown content
2. **Tool Use**: `[dot/spinner] TOOL_NAME (details)` with bold badge
3. **Tool Results**: Tool-specific rendering with status indicators
4. **Markdown**: Headers (bold/italic/underline for h1, bold for h2+), code blocks with syntax highlighting, inline code in "permission" color, blockquotes with `▎` prefix
5. **Status Line**: Dim text at bottom with `·` separators
6. **No heavy borders** — minimal structure, content-first
7. **Tables**: Box-drawing characters (`┌─┬─┐`)

## Scope

### 1. Message Display (assistant responses)
- `●` dot prefix in text color
- Full markdown rendering:
  - `# H1` → bold + italic + underline
  - `## H2`+ → bold
  - `**bold**` → bold
  - `_italic_` → italic
  - `` `code` `` → permission color (purple/cyan)
  - `> quote` → `▎` prefix + italic
  - `- item` → bullet list
  - `1. item` → numbered list
  - `---` → horizontal rule
  - Code fences → syntax highlighted with no heavy borders
- No truncation — full content always shown
- No response-level borders (content speaks for itself)

### 2. Tool Execution Display
- Spinner/dot for in-progress tools
- Tool name as **bold** badge
- Command preview in dim color
- Result output directly below
- Exit code indicator (✓/✗)

### 3. Status Line
- Dim color, bottom of screen
- Shows: model · context usage · effort time
- Uses `·` (middot) as separator

### 4. Prompt Input
- Yellow `>` prompt prefix
- Clean, no chrome

### 5. Color Palette — Tokyo Night
| Color | Hex | Claude Equivalent | Usage |
|-------|-----|-------------------|-------|
| Purple `#bb9af7` | `permission` | Inline code, accents |
| Cyan `#7dcfff` | `text` (default) | Normal text, info |
| Red `#f7768e` | `error` | Errors, failures |
| Yellow `#e0af68` | `warning` | Warnings, prompts |
| Green `#9ece6a` | `success` | Success, confirmations |
| White `#c0caf5` | `text` | Primary content |
| Comment `#565f89` | `subtle` | Metadata, dim text |

### 6. Special Characters
| Usage | Character | Unicode |
|-------|-----------|---------|
| Message dot | `●` | U+25CF |
| Blockquote bar | `▎` | U+258E |
| List bullet | `•` | U+2022 |
| Separator | `·` | U+00B7 |
| Horizontal rule | `─` | U+2500 |

## Technical Tasks

### Files to create/modify:
- `src/ui_layout.rs` — Simplify: remove heavy borders, add dot prefix
- `src/ui_markdown.rs` — Full markdown: headers, bold, italic, inline code, blockquotes, lists, tables
- `src/ui_trace.rs` — Update `print_elma_message` to use new layout
- `src/tool_calling.rs` — Update tool output to match Claude Code style
- `src/app_chat_helpers.rs` — Update status line formatting

### Delete:
- `src/ui_context_bar.rs` — Replace with simpler text-based display (Claude doesn't use a visual bar, just percentages)

## Design Principles
- **Content-first**: No decorative borders around responses
- **Minimal structure**: `● text` is enough — let markdown formatting do the work
- **Full output**: Never truncate, never hide
- **Claude Code parity**: Match the visual quality of Claude Code's TUI
- **Tokyo Night colors**: Bold, high-contrast, semantic

## Verification
1. `cargo build` clean
2. `cargo test` — all tests pass
3. Real CLI: Ask a question → see `●` + formatted markdown response
4. Real CLI: Tool execution → see `TOOL_NAME` badge + output
5. Real CLI: Status line → see dim metadata at bottom

## Acceptance Criteria
- [ ] Assistant messages render with `●` prefix
- [ ] Markdown renders fully: headers, bold, italic, code, lists, blockquotes
- [ ] Tool execution shows `TOOL_NAME` badge with command and output
- [ ] Status line shows model · context% · time
- [ ] No heavy borders around responses
- [ ] No truncation of any output
- [ ] Tokyo Night colors throughout
