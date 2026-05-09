# Task 776: Surface Compaction and Routing Decisions in Transcript

## Type
Observability / Transparency

## Severity
Medium

## Scope
UI / Orchestration

## Problem
Key architectural decisions—such as context compaction triggers and model-based routing verdicts—are currently buried in trace-level logging (`tracing::info!`) or hidden in internal state. This violates "Rule 6 — Prefer Transcript-Native Operational Visibility" in `AGENTS.md`.

## Root Cause
Early development prioritized a clean UI by hiding operational "noise," but as the system matured, these signals became critical for understanding the agent's behavior and debugging failures.

## Proposed Solution
Expose hidden decisions as collapsible transcript rows.

- Phase 1: Update `src/auto_compact.rs` to emit a UI event when compaction occurs, including tokens freed and the reason.
- Phase 2: Update `src/routing_infer.rs` to emit a UI event showing the model's routing verdict, label, and reasoning.
- Phase 3: Ensure `src/tool_loop.rs` surfaces "Stop Reasons" (like max iterations or budget exhausted) as clear transcript notices instead of generic failures.
- Phase 4: Use the existing `tui.push_meta_event` or similar mechanisms to ensure these events are visible but non-intrusive (collapsible).

## Acceptance Criteria
- [ ] Compaction events are visible in the transcript with token metrics.
- [ ] Routing decisions (e.g., "Switching to RESEARCH mode") are visible in the transcript.
- [ ] Stop reasons are clearly articulated in the UI when the tool loop terminates.

## Verification Plan
- Integration test: Run a long conversation that triggers compaction and verify the UI notice.
- Integration test: Verify that the routing verdict for a complex request is visible.

## Dependencies
None

## Notes
- Align with "Rule 6: Prefer Transcript-Native Operational Visibility".
- "Hidden process" should only be used for genuinely low-signal background tasks.
