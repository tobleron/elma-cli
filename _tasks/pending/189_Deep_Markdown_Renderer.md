# 189: Deep Markdown Renderer

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
Pending

## Priority
Medium

## Master Plan
Task 166 (Claude Code Terminal Parity Master Plan)

## Objective
Replace the primitive markdown renderer with a proper markdown parser and add code block support with syntax highlighting.

## Current State
`claude_markdown.rs` handles:
- Headers (`#`, `##`)
- Bold (`**text**`)
- Inline code (`` `code` ``)
- Bullet lists (`-`)
- Blockquotes (`>`)
- Horizontal rules (`---`)

Missing:
- Code blocks (`` ``` ``) — explicitly skipped!
- Links (`[text](url)`)
- Numbered lists
- Nested formatting
- Table alignment
- Multi-line blockquotes

## Claude Code Behavior
Claude Code uses a full markdown renderer (`<Markdown>` component) that handles all standard markdown plus:
- Code blocks with syntax highlighting via `syntect`
- Tables with aligned columns
- Links rendered as underlined text
- Images (rendered as `[Image #N]` chips in terminal)

## Implementation Plan

### Phase 1: Code Blocks
1. Detect `` ```language `` ... `` ``` `` fences
2. Parse language identifier
3. Use `syntect` for syntax highlighting (already allowed in AGENTS.md)
4. Render with background color and left border (`▎` or similar)
5. Handle code blocks that exceed viewport (add truncation)

### Phase 2: Links
1. Detect `[text](url)` pattern
2. Render text with underline + `accent_secondary` color
3. Optionally make clickable (terminal-dependent)

### Phase 3: Numbered Lists
1. Detect `1.`, `2.`, etc.
2. Render with proper indentation

### Phase 4: Full Parser
1. Integrate `pulldown-cmark` (already allowed in AGENTS.md)
2. Convert pulldown events to Ratatui `Line`/`Span` structures
3. Handle all standard markdown elements

## Files Likely Touched
- `src/claude_ui/claude_markdown.rs` — rewrite or extend
- `Cargo.toml` — add `pulldown-cmark`, `syntect` dependencies
- `src/ui/ui_theme.rs` — add code block background/border tokens

## Verification
- [ ] Snapshot test: code block with Rust syntax highlighting
- [ ] Snapshot test: table rendering
- [ ] Snapshot test: link rendering
- [ ] Snapshot test: nested list rendering
- [ ] `cargo test --test ui_parity`

## Related Tasks
- Task 166 (master plan)
- Task 170 (markdown — this is the deep implementation)

---
*Created: 2026-04-22*
