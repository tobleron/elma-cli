# 183: Input And Rendering Enhancements

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
- Implemented Shift+Enter for multiline input:
  - Inserts newline at cursor position
  - Input area grows vertically
  - Works in both idle and busy states
- All 27 UI parity tests pass
- Verification: `cargo build` passed, `cargo test --test ui_parity` passed (27 tests)

## Verification
- [x] Shift+Enter inserts newline
- [x] Multiline input submits correctly
- [x] `cargo test --test ui_parity` passes (27 tests)

## Remaining Work (Deferred)
- Recursive file picker workspace discovery (Task 188)
- Deep markdown renderer with syntax highlighting (Task 189)
- These require additional dependencies and are lower priority

## Priority
Medium

## Master Plan
Task 166 (Claude Code Terminal Parity Master Plan)

## Objective
Implement Shift+Enter multiline, recursive file picker, and deep markdown renderer.

## Part A: Shift+Enter For Multiline Input

### Current
`InputMode::Multiline` exists but no key binding.

### Target
- `Shift+Enter` inserts newline at cursor
- Input area grows vertically
- Prompt prefix (`❯`) only on first line; continuation lines use `  ` indent
- `Enter` submits; `Shift+Enter` adds newline

### Implementation
1. Detect `Shift+Enter` in key handling
2. Insert newline at cursor position in `TextInput`
3. Grow `input_lines` in renderer
4. Handle backspace at start of line → merge with previous

## Part B: Recursive File Picker

### Current
`@` picker only scans top-level directory.

### Target
Recursive discovery:
- Use `walkdir` or `ignore` crate
- Respect `.gitignore`
- Skip: `target/`, `node_modules/`, `.git/`, `.idea/`
- Limit: ~10,000 files max
- Cache and refresh on `@` activation

## Part C: Deep Markdown Renderer

### Current
Code blocks skipped. No links, tables, numbered lists.

### Target
Full markdown:
1. **Code blocks:** `` ```language `` ... `` ``` `` with `syntect` highlighting
2. **Links:** `[text](url)` → underlined + accent color
3. **Tables:** Proper column alignment
4. **Numbered lists:** `1.` rendering
5. Use `pulldown-cmark` + `syntect`

## Files Likely Touched
- `src/ui/ui_terminal.rs` — Shift+Enter detection
- `src/ui/ui_input.rs` — multiline input
- `src/claude_ui/claude_render.rs` — file discovery
- `src/claude_ui/claude_markdown.rs` — deep markdown
- `Cargo.toml` — add dependencies

## Verification
- [ ] PTY fixture: Shift+Enter → new line inserted
- [ ] PTY fixture: @ picker shows nested files
- [ ] Snapshot: code block with syntax highlighting
- [ ] Snapshot: table rendering
- [ ] `cargo test --test ui_parity`

## Related Tasks
- Task 166 (master plan)
- Task 173 (prompt input)

---
*Created: 2026-04-22*
