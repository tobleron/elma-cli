# Task 641: Status Bar Contract And Transcript Native Operational Visibility

**Status:** pending
**Priority:** HIGH
**Type:** Observability / UI Internals
**Scope:** `src/claude_ui/`, `src/event_log.rs`, `src/tool_loop.rs`, `src/app_chat_loop.rs`
**Source:** AGENTS.md rule 5/6, `_knowledge_base` event stream audit

## Summary

Keep the footer/status bar limited to model name, token count, and elapsed time, and move all operational decisions into collapsible transcript rows.

## Evidence And Gap

- AGENTS.md says execution mode, queue notices, operational notifications, routing decisions, compaction triggers, and stop reasons belong in the chat transcript.
- `FooterModel` still contains `transcript_metric` and `mode_label` fields.
- `tool_loop.rs` and `app_chat_loop.rs` record many details in trace/event logs without a guaranteed transcript row.

## Implementation Plan

1. Define a footer contract test that rejects non-core runtime metrics.
2. Add transcript notice emission for route, complexity, formula/recipe, budget, compaction, stop reason, retries, and tool discovery.
3. Store notice metadata in session artifacts for replay.
4. Remove mode/queue/hidden process text from footer rendering.

## Acceptance Criteria

- [ ] Footer contains only model, token count, and elapsed time.
- [ ] Operational visibility rows are collapsible and saved to session transcript.
- [ ] Missing route/complexity/stop reason rows fail tests.
- [ ] No trace-only operational decision is required to understand a turn.

## Verification Plan

Run a prompt that triggers at least one tool call and inspect `session.md`, `session.json`, and UI output for visible operational rows.

