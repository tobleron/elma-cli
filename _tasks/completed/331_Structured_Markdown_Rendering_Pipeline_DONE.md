# Task 331: Structured Markdown Rendering Pipeline — No ANSI Internal Path

**Status:** completed, with follow-up fix applied for width-aware Ratatui rendering  
**Depends on:** None (self-contained refactoring of existing `claude_markdown.rs`)

## Objective

Replace the monolithic `render_markdown_ratatui()` in `src/claude_ui/claude_markdown.rs` with a clean, two-phase pipeline:

```
model → pulldown-cmark parser → Vec<RenderBlock> IR → line-based renderer → Vec<Line<'static>> → Paragraph
```

The internal representation must **never** contain ANSI escape sequences. ANSI conversion is reserved for external command output and legacy non-Ratatui paths only (see `markdown_ansi.rs`).

## Architectural Constraint (Do Not Violate)

Elma renders the entire transcript as **a single `Paragraph` widget** (`claude_render.rs:885`). All messages are flattened into `Vec<Line<'static>>`, concatenated, and fed to one `Paragraph`. This is simple, reliable, and works correctly with scrolling, wrapping, and click-to-expand hit-testing.

**This task must NOT change that architecture.** Do not attempt to embed heterogeneous Ratatui widgets (`Table`, `List`) into the transcript layout. That would require per-message sub-rects, rewriting scroll logic, and breaking line-mapping for hit-testing.

Instead, tables and lists are rendered into properly formatted `Line` sequences with correct styling, alignment, and box-drawing borders. The result is still `Vec<Line<'static>>` — but generated from a structured, testable IR rather than ad-hoc string flattening.

## Current State

`src/claude_ui/claude_markdown.rs` already uses `pulldown-cmark` with `ENABLE_TABLES`, but:
- Parsing and rendering are tightly coupled in a single ~400-line function (`render_markdown_ratatui`).
- Tables are flattened into `Vec<Line>` with manual `Span` construction — column widths are computed, but borders are pipe characters (`|`) and the renderer is inline with the parser.
- Inline styles are managed with boolean flags (`in_bold`, `in_italic`, etc.) and manual `flush_pending_text` calls.
- There is no reusable intermediate representation; blocks are stored as raw markdown strings in `AssistantBlock`.

## Scope

### 1. Define the Intermediate Representation (IR)

Create a new module `src/claude_ui/markdown_blocks.rs` (or inline in `claude_markdown.rs` if small) with:

```rust
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum RenderBlock {
    Paragraph(Text<'static>),
    Heading { level: u8, content: Text<'static> },
    CodeBlock { language: Option<String>, lines: Vec<String> },
    List { ordered: bool, items: Vec<Text<'static>> },
    BlockQuote(Text<'static>),
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    Rule,
}
```

Notes:
- `Text<'static>` = `Vec<Line<'static>>` — preserves inline spans and styling.
- Code blocks store raw `Vec<String>` lines so that syntect highlighting can be applied later by the renderer.
- Tables store `Vec<Vec<String>>` so the renderer can compute column widths and emit aligned line output.

### 2. Extract the Parser Phase

```rust
pub(crate) fn parse_markdown(input: &str) -> Vec<RenderBlock>;
```

Implementation rules:
- Use `pulldown_cmark::Parser` with `Options::ENABLE_STRIKETHROUGH | ENABLE_TABLES | ENABLE_FOOTNOTES | ENABLE_TASKLISTS`.
- Maintain a style stack (or simple `Vec<Modifier>`) for inline formatting.
- Map inline events to styled `Span`s accumulated into `Line`s, then into `Text`:
  - `Strong` → `Modifier::BOLD`
  - `Emphasis` → `Modifier::ITALIC`
  - `Strikethrough` → `Modifier::CROSSED_OUT`
  - `Code` → distinct style (e.g. `fg(theme.accent_secondary)` + `Modifier::DIM`)
  - `Link { dest_url, .. }` → underlined text with `fg(theme.accent_secondary)`; optionally append URL inline.
- `SoftBreak` → space (or wrap-friendly continuation within the same `Line`)
- `HardBreak` → new `Line`
- Paragraph end → push a blank `Line` after the block (spacing is a renderer concern, but the parser may emit a separator hint).

### 3. Extract the Renderer Phase

```rust
pub(crate) fn render_blocks_to_lines(
    blocks: &[RenderBlock],
    theme: &Theme,
    width: usize,
) -> Vec<Line<'static>>;
```

This is the **only** renderer output type needed. The transcript consumes `Vec<Line<'static>>` via `Paragraph`. Do not introduce a `MarkdownWidget` enum or attempt native `ratatui::widgets::Table` embedding.

Block-to-lines mapping:

| RenderBlock | Output | Notes |
|-------------|--------|-------|
| `Paragraph(Text)` | `Text` lines directly | Wrap-friendly via `Paragraph::wrap` |
| `Heading { level, content }` | Bold `Text` with top/bottom blank lines | Level affects visual weight (e.g. H1 gets accent color) |
| `CodeBlock { language, lines }` | Syntect-highlighted lines with language header and indent prefix | Monospace style, no wrapping unless explicitly desired |
| `List { ordered, items }` | Each item prefixed with `"• "` or `"1. "`, indented by level | Items are `Text` (may contain inline styles) |
| `BlockQuote(Text)` | Dimmed fg, `BLOCKQUOTE_BAR` prefix per line | `"│ "` or `"▌ "` style prefix |
| `Table { headers, rows }` | Properly aligned lines with box-drawing borders | See table renderer requirements below |
| `Rule` | Repeated `─` span on one line | With blank lines above/below |

Table renderer requirements (critical):
- Compute per-column widths from cell content lengths.
- Use box-drawing characters: `│` columns, `─` row separators, `┼` intersections, `┌┐└┘` optional outer frame.
- Header row is bold; separator line between header and body.
- Support cell wrapping: if a cell exceeds `max_col_width` (derived from `width / num_cols`), wrap the cell text and expand the row height. This is handled by splitting the cell into multiple physical lines while keeping column alignment.
- Do **not** flatten cell boundaries. Preserve them so columns stay visually aligned.

Example output for a 2-column table:
```
┌────────┬───────┐
│ Name   │ Value │
├────────┼───────┤
│ Foo    │ 42    │
│ Bar    │ 99    │
└────────┴───────┘
```

### 4. Preserve Existing Public API

Keep these signatures stable so `claude_render.rs` does not break during the migration:

```rust
pub(crate) fn render_markdown_ratatui(text: &str) -> Vec<Line<'static>>;
pub(crate) fn render_assistant_content(content: &AssistantContent, width: usize) -> Vec<Line<'static>>;
pub(crate) struct AssistantContent { pub raw_markdown: String, pub blocks: Vec<AssistantBlock> }
```

Internally, `render_markdown_ratatui` should become a thin wrapper:
```rust
pub(crate) fn render_markdown_ratatui(text: &str) -> Vec<Line<'static>> {
    let blocks = parse_markdown(text);
    render_blocks_to_lines(&blocks, current_theme(), 80)
}
```

`AssistantBlock` can remain as-is for now (it is a higher-level semantic classifier). The new `RenderBlock` IR sits *below* it, inside the rendering pipeline.

### 5. Update `markdown_ansi.rs` Boundary

Add a clarifying doc comment:
```rust
//! ANSI conversion is ONLY for external command output and legacy stdout paths.
//! LLM Markdown → Ratatui must go through the structured pipeline in claude_markdown.rs.
```

Ensure no code path accidentally calls `render_markdown_to_ansi` and then re-parses ANSI back into Ratatui spans.

### 6. Testing Requirements

Write unit tests in `src/claude_ui/claude_markdown.rs` (or a new `markdown_blocks_tests.rs`):

- **Bold / italic nesting:** `**bold *and italic***` produces correct `Modifier` stack.
- **Inline code:** `` `code` `` gets the code style span, preserving surrounding text order.
- **Headings:** `# H1` and `## H2` produce `Heading` blocks with correct `level`.
- **Lists:** ordered (`1. 2.`) and unordered (`- *`) produce `List` blocks with correct `ordered` flag and item `Text`.
- **Tables:** header + multiple rows parsed into `Table` block; cell boundaries preserved; rendered output has aligned columns with box-drawing borders.
- **Code blocks:** fenced and indented blocks produce `CodeBlock` with language tag and raw lines.
- **Blockquotes:** `> quote` produces `BlockQuote` with correct text.
- **Soft vs hard breaks:** verify space vs new-line behavior.
- **Rule:** `---` produces a separator line.

## Anti-Patterns To Avoid

- **Do NOT introduce ANSI into the IR.** If you find yourself generating `\x1b[` strings inside `parse_markdown` or `render_blocks_to_lines`, stop.
- **Do NOT bloat `RenderBlock` with layout metrics.** Width, alignment, and wrapping are renderer concerns.
- **Do NOT merge parsing and rendering again.** The parser should not know about terminal width; the renderer should not re-parse Markdown.
- **Do NOT change the transcript architecture.** Keep the single-`Paragraph` layout in `claude_render.rs`. Do not try to embed `Table` or `List` widgets into the transcript.
- **Do NOT change `prompt_core.rs` or `TOOL_CALLING_SYSTEM_PROMPT`.** This is a UI-layer refactor.
- **Do NOT remove `markdown_ansi.rs`.** It serves external command output and file export paths legitimately.

## Verification

```bash
cargo build
cargo test -- claude_markdown
```

After implementation, run a manual smoke test: start Elma, ask the model to output a Markdown table, and verify it renders with proper column alignment and box-drawing borders (not flattened text).

## Success Criteria

- [ ] `parse_markdown` and `render_blocks_to_lines` exist as separate, testable functions.
- [ ] `RenderBlock` enum covers all required block types.
- [ ] Tables are rendered with properly computed column widths and box-drawing borders (aligned `Vec<Line>`, not raw pipe characters).
- [ ] Inline styles use `Modifier` (BOLD, ITALIC, etc.) on `Span`s, not ANSI strings.
- [ ] Existing `render_markdown_ratatui` and `render_assistant_content` signatures remain stable.
- [ ] All existing tests in `claude_markdown.rs` still pass.
- [ ] New unit tests cover: bold/italic nesting, inline code, headings, lists, tables, code blocks, blockquotes, breaks, rules.
- [ ] No ANSI escapes appear in the internal Markdown→Ratatui pipeline.
- [ ] Transcript architecture in `claude_render.rs` is unchanged (still single `Paragraph`).

## Outcome

A clean, extensible pipeline that produces correct layout, especially for tables and complex structured content, while respecting Elma's existing terminal UI architecture.

## Follow-up Verification

The initial implementation created the structured Markdown -> RenderBlock -> Ratatui Line pipeline and kept ANSI out of the internal Ratatui path. A later verification found two rendering gaps:

- `render_markdown_ratatui()` still used an 80-column compatibility wrapper from width-aware call sites.
- The transcript viewport sliced logical lines before Ratatui wrapping, so the visible bottom could stop before the saved final answer.

The follow-up fix makes assistant Markdown rendering width-aware and pre-wraps transcript lines before viewport slicing.
