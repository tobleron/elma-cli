# Claude Code Terminal Parity Specification

**Source of Truth**: `_stress_testing/_claude_code_src/`
**Target**: Elma CLI interactive mode
**Version**: 1.0 (Task 167)

---

## 1. Layout & Overall Architecture

Claude Code's terminal UI is sparse, message-first, and deliberately minimal:

- **No persistent header strip** — Elma branding is absent or minimal.
- **No activity rail** — progress and streaming are shown inline with messages.
- **No boxed input composer** — input is a single-line prompt `> ` with no surrounding borders.
- **No persistent context bar** — token usage is shown conditionally or omitted in default view.
- **Message rows fill the screen** — transcript scrolls, latest messages always visible.
- **Modal/overlay pattern** — slash picker, file picker, help menu appear as transient overlay panels that capture input and can be dismissed.

**Source References**:
- Layout structure: `components/App.tsx`, `components/Messages.tsx`
- No borders/rails: Message components render plain lines, not framed boxes.

---

## 2. Message Row Types & Prefixes

Each message type uses a consistent two-character prefix column:

| Role | Prefix | Source |
|------|--------|--------|
| User | `> ` (angle bracket + space) | `components/messages/UserTextMessage.tsx` — renders `> ` on first line, `  ` (two spaces) on continuation lines |
| Assistant | `● ` (black circle + space) | `components/MessageRow.tsx` → `Message` → `AssistantTextMessage`, line 362 shows `●` bullet |
| Thinking (collapsed) | `∴ ` (therefore symbol + space) | `components/messages/AssistantThinkingMessage.tsx` line 44: `"∴ Thinking"` |
| Thinking (expanded) | `    ` (four spaces indent) | Same file line 69: `<Box paddingLeft={2}>` then markdown |
| Tool start | `  ▸ ` (two spaces + black right-pointing triangle + space) | `src/ui_render.rs` line 382 in Elma current; Claude equivalent: `components/AssistantToolUseMessage.tsx` renders tool name with a leading indicator |
| Tool result (success) | `  ✓ ` | `src/ui_render.rs` line 398 shows checkmark; Claude Code uses similar `✓` |
| Tool result (failure) | `  ✗ ` | `src/ui_render.rs` line 400 shows cross; Claude Code uses `✗` |
| System/error | `⚠ ` or special inline formatting | Various service error messages |

**Continuation lines** are indented to align with the message body (typically 2 or 4 spaces).

**Spacing**: One blank line between messages.

---

## 3. Assistant Text Rendering

- Full markdown support via `StreamingMarkdown` / `Markdown` component.
- Inline rendering: code blocks, lists, bold/italic all work.
- First line prefixed `● `; subsequent lines indented `  ` (two spaces).
- Empty messages render `● (empty)` dimmed.
- **No borders** around text blocks.

**Source**: `components/messages/AssistantTextMessage.tsx`, `components/Markdown.tsx`.

---

## 4. Thinking Display (`∴ Thinking`)

Thinking blocks have two modes:

1. **Normal mode (default)**: Collapsed to a single line:
   ```
   ∴ Thinking (ctrl+o to expand)
   ```
   - Uses dim + italic styling.
   - `Ctrl+O` expands it inline.

2. **Transcript/Verbose mode**: Expanded with full markdown content:
   ```
   ∴ Thinking…
       <thinking content in dim markdown, indented 4 spaces>
   ```
   - Shows ellipsis `…` on first line or just the content depending on state.
   - Expanded thinking appears in transcript view (`screen === "transcript"`).

**Keybindings**:
- `Ctrl+O` toggles transcript mode (shows/hides all thinking blocks).
- Thinking is hidden in transcript mode unless explicitly shown.

**Source**: `components/messages/AssistantThinkingMessage.tsx`, lines 39-58.

---

## 5. Tool Use UX: Start → Progress → Result

Tool use blocks follow a three-phase visual pattern:

### Phase 1 — Tool Start
```
  ▸ tool_name [command preview if available]
```
- Indented two spaces.
- Tool name in bold (color depends on theme).
- Optional command preview shown dimmed.

**Source**: `components/AssistantToolUseMessage.tsx` renders tool start; Elma equivalent in `ui_render.rs` line 379.

### Phase 2 — In-Progress
While the tool runs:
- A **spinner** appears next to the tool name (blinking black circle `○` → `●` or animated dots).
- Progress messages (from `ToolProgressData`) may appear below, indented.
- The tool row stays active until result arrives.

**Source**: `components/ToolUseLoader.tsx` shows `BLACK_CIRCLE` with blink; progress messages handled by `HookProgressMessage`.

### Phase 3 — Result
On completion:
```
  ✓ tool_name (1.2s)
      output line 1
      output line 2
      ...
      ... (N more lines)
```
- Success: `✓` in green; Failure: `✗` in red.
- Duration shown in parentheses if >0ms: `123ms` or `1.2s`.
- Output limited to ~15 lines; additional lines collapsed with `... (N more lines)`.
- Output text is dimmed and indented 4 spaces.

**Source**: `ui_render.rs` lines 389-441 (Elma current). Claude Code behavior matches except for exact line limits.

---

## 6. Permission/Waiting States

When a tool requires permission:
1. UI shows a **modal overlay** (not inline).
2. Modal presents options: `[Y]es`, `[A]lways`, `[N]o` or equivalent.
3. Modal captures all input until dismissed.

Modal implementation: `components/permissions/PermissionRequest.tsx` (not directly audited but referenced). The TUI equivalent should use a centered modal overlay that:
- Clears screen or dims background.
- Shows title and options.
- Captures single-key input (Y/A/N or left/right arrows).
- Dismisses on selection.

**Important**: This differs from Elma's previous prompt-based permission gates. The modal is part of the render loop, not a blocking stdin read.

---

## 7. Slash Command Fuzzy Picker (`/`)

Triggered when input starts with `/`.

### Behavior
- Input's prefix `/` activates the **FuzzyPicker** component.
- Picker appears **below the input line** as an overlay panel.
- Contains:
  - Title line: "Commands" or custom title.
  - Search box: the typed query (already in input, duplicated or focused in picker).
  - Item list: up to 8 visible items (scrollable if more).
  - Preview pane (optional): on right side if terminal width ≥ 120 cols, else below list.
  - Hints row: "↑↓ to select, Enter to choose, Esc to cancel" plus Tab if secondary action exists.

### Navigation
- `↑`/`↓`: move selection.
- `Enter`: selects the focused item (primary action).
- `Tab`: triggers secondary action if defined (`onTab` prop); otherwise acts as Enter.
- `Shift+Tab`: optional secondary or reverse.
- `Esc`: dismisses picker and returns to normal input.

### Filtering
- Picker receives `onQueryChange` callback; caller filters items reactively.
- Matches highlight not specified in source, but typical fuzzy match behavior assumed.

### Layout Constraints
- `CHROME_ROWS = 10` includes: padding, title, divider, search box (3 rows), hints (1+ rows).
- Maximum visible items capped to `rows - CHROME_ROWS` so picker never exceeds terminal height.
- If terminal too small, visibleCount reduces and scrolling becomes necessary.
- Picker positions direction `'down'` (items below input) or `'up'` (above, atuin-style).

**Source**: `components/design-system/FuzzyPicker.tsx`, lines 68-120, 145-210.

---

## 8. File Quick-Open (`@`)

Triggered by `@` prefix or `ctrl+shift+p` (Quick Open dialog).

### Behavior
- Uses `FuzzyPicker` foundation with file-specific rendering.
- Shows file paths relative to CWD, truncated in the middle if needed (`truncatePathMiddle`).
- Preview pane shows:
  - First ~20 lines of the file.
  - Syntax-highlighted if language detected.
  - Updates asynchronously on focus change; aborts stale preview requests.

### Dimensions
- `VISIBLE_RESULTS = 8`
- `PREVIEW_LINES = 20`
- `visibleResults = min(8, max(4, rows - 14))`
- Preview on right if `columns >= 120`, else below list.

**Source**: `components/QuickOpenDialog.tsx`, lines 21-39, 67-100.

---

## 9. Prompt Input Footer & Help Menu

Below the input line, a footer displays contextual hints:

- Shows suggestion hints when autocomplete or slash commands active.
- Shows mode indicator (e.g., "multiline", "vim").
- Shows status of API key, model loading, etc.
- Has a **help menu** (triggered by `?` or automatically shown on focus in some modes).

### Help Menu Contents
Standard hints include:
- `ctrl+o` — toggle transcript
- `ctrl+t` — toggle task list
- `ctrl+_` / `ctrl+shift+-` — undo
- `ctrl+s` — stash
- `shift+tab` — cycle mode
- `alt+p` — model picker
- `alt+o` — fast mode toggle
- `ctrl+g` — open in external editor
- `ctrl+shift+f` — global search
- `ctrl+shift+p` — quick open
- `ctrl+r` — history search
- `enter` — submit
- `esc` — cancel

Platform-specific variations exist (image paste: `ctrl+v` or `alt+v`).

**Source**: `components/PromptInput/PromptInputFooter.tsx`, `components/PromptInput/PromptInputHelpMenu.tsx` lines 13-100.

---

## 10. Task List (Todo Tool Display)

Invoked via `ctrl+t` or when Todo tool emits structured tasks.

### Display Behavior
- Shows up to 10 tasks (or fewer on small terminals: `max(3, rows - 14)`).
- Tasks sorted by ID ascending.
- Each task shows: `{id}. {description}` with a status indicator:
  - Pending: `○` or ` `
  - In progress: `◐` or spinner (when the agent working on it)
  - Completed: `✓` (green check)
- **Recent completion fade**: Completed tasks remain visible for `30_000ms` (30s), then automatically disappear from the list. This uses a timed re-render to remove expired tasks.
- If no tasks, nothing shown.

### Integration
The task list is conditionally rendered based on `isTodoV2Enabled()` (feature flag) and only appears when there are active or recently completed tasks.

**Source**: `components/TaskListV2.tsx`, lines 48-85, 86-100.

---

## 11. Compact Summary & Boundary

When `/compact` is used or auto-compact triggers:

### Compact Boundary Message
```
✻ Conversation compacted (ctrl+o for history)
```
- Appears as a thin divider line in the transcript.
- Dimmed text.
- Reminds user that earlier messages are summarized and how to expand.

**Source**: `components/messages/CompactBoundaryMessage.tsx`.

### Compact Summary
When user messages are summarized (auto-compact), the original user message is replaced with a summary card:

```
● Summarized conversation
  Summarized 12 messages up to this point
  Context: "user's original intent…"
  ctrl+o to expand history
```
- A small circle (`●` or `○`) icon on the left.
- Title in bold: "Summarized conversation".
- Metadata line: "Summarized N messages up to this point" (or "from this point").
- Optional user context quote in italics/dim.
- Hint: `ctrl+o to expand history`.

**Source**: `components/CompactSummary.tsx`, lines 34-74.

**Transcript Mode**: In transcript view (`ctrl+o` held or toggled), the original unsummarized messages are shown instead of the summary card.

---

## 12. Status Line (Bottom Bar)

Status line is **conditional**, not always visible.

**When shown**:
- `statusLineShouldDisplay(settings)` returns `true`.
- Hidden in KAIROS/Brief mode (assistant mode) because session info belongs to daemon.
- Shown in regular interactive REPL mode.

**What it displays**:
- Model name (runtime model, possibly with color).
- Workspace: current directory + project directory.
- Session name/session ID.
- Token usage: current / max, with a colored progress bar.
- Permission mode (auto/plan/confirm).
- Cost/utilization metrics (if configured).
- Time/effort indicators.

**Source**: `components/StatusLine.tsx`, lines 30-80.

**Elma mapping**: Existing `FooterMetrics` in `ui_render.rs` is similar but currently always shown; should become conditional per Claude behavior.

---

## 13. Transcript Mode (`Ctrl+O`)

A global toggle that affects how the transcript is rendered:

- **Off (default)**: Thinking blocks collapsed to `∴ Thinking` line; compact summaries shown instead of original messages; recent tool outputs visible.
- **On**: Full thinking content shown inline; original messages shown instead of summaries; shows entire conversation history without compaction.

Acting: `app:toggleTranscript` command, bound to `Ctrl+O`.

**Implementation pattern**: Messages utilities provide `getMessagesAfterCompactBoundary` and `shouldShowThinking`. The UI rebuilds the visible transcript from these two flags.

**Source**: `utils/messages.js`, `components/MessageRow.tsx` passes `isTranscriptMode` prop, `components/messages/AssistantThinkingMessage.tsx` line 39.

---

## 14. Exit & Cancellation

### Double-press semantics
- **`Ctrl+C`**: First press clears input (if non-empty). Second press within a short window (≈1s) sends `SIGINT` or triggers `app:interrupt` → exits/cancels current operation.
- **`Ctrl+D`**: First press if input empty sends EOF → exits. If input has content, behaves like `Enter` (submits). Double-press within window exits immediately.

Implementation uses `useDoublePress` hook or similar time-based state.

**Source**: `keybindings/defaultBindings.ts`:
```ts
'ctrl+c': 'app:interrupt',
'ctrl+d': 'app:exit',
```

Modal cancellation: `Esc` dismisses modal/picker.

---

## 15. Bash Mode (`!`) & Background Mode (`&`)

Elma-specific: these are **single-character command modes** entered at the prompt:
- Type `!` followed by a shell command runs it directly (bypasses agent).
- Type `&` at end of message runs request in background (user can continue typing).

These are **not** observed in Claude Code source (which lacks these modes), but must be preserved as Elma-only language features per Task 166's "Intentional Differences" section.

**Implementation**: Currently in `src/auto_compact.rs` or `src/tool_calling.rs` background handling; shell commands via `shell` tool. These should be retained but styled consistent with the new UI.

---

## 16. Multi-line Input

Triggered by `Ctrl+J` (not `Enter`):
- Inserts a newline into the input buffer.
- Prompt re-renders with multiple lines (up to 3 visible lines typically).
- Input box height expands to fit content (up to max 3 lines), then scrolls internally.

**Source**: `keybindings/defaultBindings.ts` global bindings list does not include `ctrl+j` explicitly; it's handled in the Input component's keymap as `chat:multiline` or raw `Ctrl+J` newline insertion in the TextInput layer.

Elma already has `Ctrl+J` in `ui_terminal.rs` line 517-520.

---

## 17. History Navigation

- `↑` (Up): previous history entry.
- `↓` (Down): next history entry.
- `Ctrl+R`: incremental history search (like Bash reverse-i-search).
- History stored in `sessions/history.txt`.

**Source**: `keybindings/defaultBindings.ts`:
```ts
up: 'history:previous',
down: 'history:next',
'ctrl+r': 'history:search',
```

---

## 18. Theme & Color Tokenization

Claude Code uses a token-based theme system. Colors are semantic, not hardcoded.

**Default theme (Claude)**:
- Background: dark/black (`#000000` or very dark gray).
- Foreground text: white/off-white.
- Primary accent: vibrant blue or purple for highlights.
- Secondary accent: cyan/green for success states.
- Dim/gray: secondary text, borders, disabled UI.

**Elma parity theme** (from Task 166):
- Background: **black**.
- Primary text: **white**.
- Greys: metadata, separators, disabled, inactive.
- **Primary accent: high-contrast Pink**.
- **Complementary accent: high-contrast Cyan**.

Colors must be **tokenized** so future themes replace Pink/ Cyan without touching renderers.

**Tokens needed** (minimum):
- `fg` — primary text (white)
- `fg_dim` — secondary text (grey)
- `accent_primary` — Pink (used for `>` prompt, selected items, attention)
- `accent_secondary` — Cyan (used for `●` assistant prefix, tool result checkmark, info)
- `success` — green
- `error` — red
- `warning` — yellow
- `border` — grey for picker borders (if any)
- `select_bg` — selected item background (inverted or highlighted)

**Source**: Theme system in `src/utils/theme.ts` equivalent; Elma needs a new `ui_theme.rs` with token struct.

**Explicitly NOT Gruvbox**: Current `ui_theme.rs` uses Gruvbox color constants; these must be replaced.

---

## 19. Message Grouping & Collapsing

Certain tool uses are automatically **grouped/collapsed**:
- Multiple `read` or `search` calls within a short window collapse into a "Reading…" or "Searching…" group with a spinner.
- When complete, the group shows summary like `✓ Read 15 files` (collapsed).
- Can be expanded to show individual results if needed (via message actions, optional).

This behavior is in `utils/collapseReadSearch.js` and `utils/groupToolUses.js`. For initial parity, Elma can show individual tool results without grouping, but the loading spinner behavior should match.

**Simplified initial pass**: Show each tool use individually with its own start→progress→result cycle. Grouping can be deferred to a later polish task.

---

## 20. Modal & Overlay Lifecycle

Modals (permissions, pickers, help menu) must:
1. **Not block** the TUI event loop — handled via state machine in render loop.
2. **Dismiss on `Esc`** (unless inside a text input field where `Esc` clears input first).
3. **Restore cursor position** in input after dismiss.
4. **Redraw entire screen** while modal active (clear + overlay render).

Elma's existing `ModalState` in `ui_state.rs` is close but uses raw stdin blocking. Must convert to non-blocking, state-driven modal rendering.

---

## 21. Behavior Differences from Claude Code (Allowed)

- **Rust implementation** — not React/Ink.
- **Elma identity** — model names, endpoint display can say "Elma" not "Claude".
- **Single local model only** — no multi-agent, no cloud teams, no telemetry, no Buddy/teleport/voice.
- **Small-model-first UX** — may show explicit progress/effort badges, but order and style must still match Claude's look.

All other visual and interaction patterns must approximate the reference as closely as practical.

---

## 22. Non-Negotiables (Aggressive Compliance)

These old Elma UI patterns **must be removed** during parity work:

- ❌ Header strip with `Elma` logo + workspace bar
- ❌ Activity rail with spinner bar separate from message stream  
- ❌ Boxed input (borders around composer)
- ❌ Persistent context bar below input
- ❌ Gruvbox-only color scheme
- ❌ Mixed `println!` output during TUI operation
- ❌ Blocking raw stdin prompts
- ❌ Modal overlays that don't redraw the screen
- ❌ Stale footer after resize/interrupt

Replace with Claude-like alternatives even if refactoring is extensive.

---

## 23. Unchanged Elma Philosophy

Keep these internal systems even though UI changes:
- Context management & auto-compaction (already implemented).
- Token budgeting.
- Tool result persistence to disk (behind-the-scenes, UI-agnostic).
- Small-model reliability features.

UI parity governs **visible behavior only**, not internal planning or orchestration.

---

## 24. Reference File Map

| Behavior | Source File |
|----------|-------------|
| Top-level app shell | `components/App.tsx` |
| Message list rendering | `components/Messages.tsx`, `components/MessageRow.tsx` |
| Individual message types | `components/messages/*.tsx` |
| Thinking block UI | `components/messages/AssistantThinkingMessage.tsx` |
| Tool use UI | `components/messages/AssistantToolUseMessage.tsx`, `components/ToolUseLoader.tsx` |
| Prompt input | `components/PromptInput/PromptInput.tsx` |
| Footer hints | `components/PromptInput/PromptInputFooter.tsx`, `PromptInputHelpMenu.tsx` |
| Fuzzy picker base | `components/design-system/FuzzyPicker.tsx` |
| File quick-open | `components/QuickOpenDialog.tsx` |
| Task list | `components/TaskListV2.tsx` |
| Status line | `components/StatusLine.tsx` |
| Compact summaries | `components/CompactSummary.tsx`, `messages/CompactBoundaryMessage.tsx` |
| Keybindings | `keybindings/defaultBindings.ts` |
| Commands registry | `commands.ts` |
| Query / streaming pipeline | `query.ts` |

---

## 25. Golden Fixture Requirements

The test harness must capture these **snapshot scenarios**:

1. **Startup** — First screen: no messages, `> ` prompt, footer hints visible.
2. **Normal prompt entry** — User types a line, hits Enter; message appears with `> ` prefix.
3. **Streaming thinking** — Model emits thinking block: shows `∴ Thinking` line.
4. **Final assistant response** — `●` prefixed markdown renders and completes.
5. **Tool execution** — Tool start → spinner progress → ✓ result with output (15+ lines to test truncation).
6. **Slash command picker** — Type `/` → picker overlay with list, keyboard selection, hints.
7. **File picker (@)** — Type `@` → file fuzzy list with preview pane on wide terminals.
8. **Todo list display** — `Todo` tool populates; task list appears with checkmarks on completion.
9. **Compact boundary** — `/compact` invoked; `✻ Conversation compacted` line appears; summary card replaces earlier messages.
10. **Transcript expansion** — `Ctrl+O` held/pressed; thinking blocks expand.
11. **Exit sequences** — `Ctrl+D` or double `Ctrl+C` exits cleanly, terminal restored.

Each fixture must be reproducible **without a live model**. Use:
- A fake OpenAI-compatible server returning canned SSE streams.
- Or a "fixture mode" where Elma replays transcript events from a YAML/JSON script.

**Output capture**: Spawn `elma` in a pseudo-terminal (`portable-pty`), drive keystrokes, capture raw VT100 stream, normalize ANSI colors to tokens (strip in non-style areas, preserve color tokens where style parity is checked), save with `insta`.

---

## 26. Implementation Order (from Task 166)

1. **T167** (this task) — Create spec + harness.
2. **T179** — Hang triage; fix blocking/stale UI before major rework.
3. **T168** — Tokenized Pink/Cyan theme.
4. **T169** — Renderer shell that draws Claude-layout (no boxes, no rails).
5. **T170** — Message row components, markdown rendering, transcript math, compact boundary.
6. **T171** — Streaming pipeline (thinking → text → tool uses).
7. **T172** — Tool UX (start → progress → result + permission modal).
8. **T173** — Prompt input, slash picker, file picker, command modes.
9. **T174** — Todo tool → task list integration.
10. **T175** — Compact logic, status line (conditional), notifications in footer.
11. **T176** — Session lifecycle (clear/resume/exit UX).
12. **T177** — Remove legacy UI modules (`ui_terminal.rs` old frames, etc.).
13. **T178** — End-to-end stress tests; all fixtures green.

---

## 27. Acceptance Criteria Summary

- Spec document completed with source citations.
- Test harness in `tests/ui_parity/` with pseudo-terminal driver.
- At least 9 snapshot fixtures covering core flows.
- Fixtures run deterministically without network/model.
- Subsequent tasks implement to match these fixtures.

---

**END OF SPECIFICATION**
