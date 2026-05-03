# 584 — Add TUI Rendering Tests with insta Snapshots

- **Priority**: Medium
- **Category**: Testing
- **Depends on**: 558 (decouple SSE from TUI)
- **Blocks**: None

## Problem Statement

The TUI layer (`src/ui/`, `src/claude_ui/`) has no automated rendering tests. TUI regressions (misaligned text, missing elements, incorrect colors) are caught only through manual testing. This is especially risky because the TUI has undergone extensive changes (Tasks 167-183 document a complete terminal parity overhaul).

The `vt100` and `insta` crates are listed as dev-dependencies (Cargo.toml:93-94) with a note "UI parity test harness" but may not be fully utilized.

## Why This Matters for Small Local LLMs

The TUI is the primary interface for users interacting with Elma. Rendering bugs degrade the user experience and make it harder to understand what the agent is doing — which is critical when working with small models that require more oversight.

## Recommended Target Behavior

Add insta snapshot tests for key TUI states:

1. **Empty state**: No messages, just prompt input
2. **Single message**: User message displayed
3. **Assistant thinking**: Thinking indicator active
4. **Assistant response**: Text response displayed
5. **Tool execution**: Tool started/finished indicators
6. **Tool result collapsed**: Collapsed tool result row
7. **Tool result expanded**: Expanded tool result content
8. **Error state**: Error message displayed
9. **Compaction boundary**: Compact boundary indicator
10. **Stop notice**: Budget/stagnation stop notice
11. **Status bar**: Footer with model name, tokens, time
12. **Multi-turn**: Several back-and-forth messages

### Test Approach

Use `vt100` to simulate a terminal and capture rendered output:

```rust
#[test]
fn test_empty_state_rendering() {
    let mut parser = vt100::Parser::new(80, 24, 0);
    let mut tui = TestTerminalUI::new(80, 24);
    tui.render(&mut parser);
    insta::assert_snapshot!(parser.screen().contents());
}
```

## Source Files That Need Modification

- `src/ui/` — May need test hooks for rendering capture
- `src/claude_ui/` — May need test hooks
- `tests/fixtures/ui_parity/` — Existing directory, populate with snapshot fixtures

## New Files/Modules

- `tests/ui_render_tests.rs` — Rendering test suite
- `tests/fixtures/ui_parity/*.snap` — insta snapshot files

## Step-by-Step Implementation Plan

1. Audit existing `vt100` + `insta` setup in the codebase
2. Create `TestTerminalUI` or test helper that renders to a `vt100::Parser`
3. Write snapshot tests for each key TUI state
4. Run `cargo insta review` to accept initial snapshots
5. Add to CI: `cargo test --test ui_render_tests`

## Recommended Crates

- `vt100` — already a dev-dependency
- `insta` — already a dev-dependency

## Acceptance Criteria

- 12+ TUI state snapshot tests
- Tests run in CI
- Snapshot changes require explicit review
- Tests cover: messages, thinking, tools, errors, footer, compaction

## Risks and Migration Notes

- Snapshot tests are sensitive to terminal size changes. Use fixed terminal dimensions.
- Color testing requires ANSI-aware comparison. `insta` supports this.
- UI changes will require snapshot updates — this is intentional (catches unintended changes).
