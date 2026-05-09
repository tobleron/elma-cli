# Task 636: UI Transcript Virtualization Render Cache And Per-Frame Budget

**Status:** pending
**Priority:** CRITICAL
**Type:** Performance / UI Internals
**Scope:** `src/claude_ui/`, `src/ui/ui_wrap.rs`, `src/token_counter.rs`, session transcript writes
**Source:** old active tasks 626-634, completed performance notes, `_knowledge_base` UI/event scans

## Summary

Make transcript rendering scale to large sessions without per-frame re-wrapping, token recounting, full allocation churn, or synchronous disk work.

## Evidence And Gap

- The deleted active tasks 626-634 targeted transcript caching, redraw rate limits, per-frame token counts, system monitor background work, allocations, input wrapping, async transcript writes, message memory caps, and permission polling.
- `ClaudeRenderer` still owns cached input wrapping and transcript buffers directly, which should move into dedicated caches/services.
- Long traces under `sessions/` show large session artifacts where repeated full rendering makes debugging and interactive use fragile.

## Implementation Plan

1. Add stable message ids and cache wrapped/rendered lines by `(message_id, width, expansion_state)`.
2. Render only the visible transcript window plus overscan; keep search and click mapping derived from virtual rows.
3. Maintain incremental token counts in message metadata instead of recounting content on draw.
4. Move transcript markdown/text disk flushes behind a bounded async writer with explicit flush on shutdown.
5. Cap retained in-memory output for very large tool results while keeping artifact references.

## Acceptance Criteria

- [ ] Draw cost is proportional to visible rows, not total transcript size.
- [ ] Token counters update incrementally and never rescan full transcript every frame.
- [ ] Disk flush failures surface as transcript-visible notices and trace entries.
- [ ] Snapshot tests cover wide/narrow terminal widths and long transcripts.

## Verification Plan

Add a synthetic 5,000-message transcript benchmark/test and verify no visible behavior regression with `cargo test ui`.

