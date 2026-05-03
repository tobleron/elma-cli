# Task 540: Stop Reasons In Transcript (AGENTS.md Rule 6)

**Status:** pending
**Priority:** HIGH
**Source:** Session analysis s_1777805162_306413000 (2026-05-03)
**Problem:** P4 — Very High Confidence
**Rule:** AGENTS.md Rule 6 (Prefer Transcript-Native Operational Visibility)

## Summary

Stop reasons, stagnation events, and budget warnings are currently written only to `trace_debug.log`. They are invisible in the user-facing transcript (`session.md`). AGENTS.md Rule 6 requires these to surface as collapsible transcript rows. This is explicitly called out as "critically important and not currently applied properly."

The session ended twice without any in-transcript explanation. The user has no way to know whether Elma finished, hit a limit, or stagnated.

## Evidence

- `trace_debug.log` line 69: `stopping reason=respond_abuse` — trace only, not in transcript
- `trace_debug.log` line 139: second `stopping reason=respond_abuse` — same
- `trace_debug.log` line 52: budget trust notice emitted — also trace only
- `session.md`: contains no stop reason rows, no budget rows, no stagnation rows

## Implementation Plan

1. Identify where stop reasons are recorded in the execution pipeline
2. At each stop decision point, emit a transcript event of type `system_event` with:
   - `kind`: `stop_reason | stagnation | budget_warning | tool_failure`
   - `message`: human-readable description (e.g., "Stopped: stagnation limit reached after 5 duplicate tool calls")
   - `collapsible: true`
3. In the TUI renderer, display these events as a collapsible grey row (metadata color per theme)
4. In `session.md` writer, serialize these events as quoted blockquote rows: `> ⚠️ Stopped: stagnation_limit`
5. Apply to: stop reasons, budget caution/danger threshold crossings, `respond_abuse`, `tool_failure_stagnation`, memory gate skips

## Success Criteria

- [ ] Every session stop appears in the transcript as a visible, collapsible row
- [ ] Budget warnings appear in the transcript at each threshold crossing
- [ ] Tool failure stagnation appears in the transcript
- [ ] No stop event is trace-only
- [ ] `session.md` written to disk reflects all operational events

## Verification

```bash
cargo build
cargo test
# Run a session that stagnates
# Open sessions/<id>/session.md
# Verify stop reason row is present
```
