# Task 385: Persist Finalized Summaries As Markdown

**Status:** Pending
**Priority:** LOW
**Estimated effort:** 1-2 days
**Dependencies:** None
**References:** _masterplan.md Task 334, user requirement for markdown as an optional artifact format

## Problem

Session summaries exist only in the chat transcript and are lost when sessions age out. Users cannot reference, revise, or build documentation from past session summaries.

## Objective

Add a `summaries/` folder inside each session directory. Every finalized summary is persisted as an individual `.md` file with timestamp, content, and optional metadata. This is an artifact/export path only; ratatui terminal output remains plain text by default.

## Implementation Plan

### Phase 1: Summary Persistence Hook

Identify where finalized summaries are emitted (likely `intel_units_final_summary.rs` or `effective_history.rs`). Add a post-processing step that writes the summary to disk:

```
sessions/s_{id}/summaries/
  2026-05-01_21-05-18_summary_1.md
  2026-05-01_21-05-20_summary_2.md
```

### Phase 2: Format

```markdown
---
timestamp: 2026-05-01T21:05:18Z
session: s_1777658707_425792000
model: Huihui-Qwen3.5-4B
---

[Summary content — exactly what was displayed in transcript]
```

If the transcript summary was rendered as plain text, convert it to simple markdown only for the file artifact. Do not make markdown a prerequisite for terminal display.

### Phase 3: Error Handling

- Disk full → log warning, continue session
- Permission error → log warning, continue session
- Directory missing → create it
- Non-UTF8 content → skip (summaries should be UTF-8)

## Files to Modify

| File | Change |
|------|--------|
| `src/session_write.rs` | Add `write_summary_markdown()` function |
| `src/intel_units/intel_units_final_summary.rs` | Call persistence hook after each summary |

## Verification

```bash
cargo build
cargo test summary
```

**Manual**: Run a session, produce at least 2 summaries. Verify `.md` files appear in the session's `summaries/` folder with correct timestamps and content.
