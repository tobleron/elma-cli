# Task 743: Enforce Transcript-Native Operational Visibility For Planning And Stops

## Type

Architecture / Observability / Rule 6 Compliance

## Severity

High

## Scope

Transcript rendering, UI, stop policy, tool loop

## Problem

**AGENTS.md Rule 6** mandates:

> "Budgeting, routing/formula choice, compaction, stop reasons, and hidden processes must surface as collapsible transcript rows. Do not bury these in trace-only state, debug logs, or hidden metadata. The transcript is the single source of truth for what happened during a session."

**This rule is critically important and is not currently applied properly.**

Current behavior:
- Routing decisions are written to `trace()` (a file log), not the user-facing transcript
- Stop reasons are written to `trace()`, not the transcript
- Compaction triggers are invisible to the user
- Budget decisions (iteration counts, token usage) are only in the status bar footer
- Formula selection is invisible
- The `app_chat_loop.rs` comment even acknowledges: "Route classification is no longer needed" — but this means the user has **zero visibility** into why the agent chose a particular execution path

Evidence from codebase:
```rust
// In app_chat_loop.rs
trace(&runtime.args, &format!("planning_source=maestro ladder_level={:?}", ladder.level));
trace(&runtime.args, &format!("intent_annotation={}", ...));
```

These `trace()` calls go to a file, not the transcript UI. The user sees:
- "Thinking..."
- "Executing..."
- Final answer

They do NOT see:
- "Classified as MULTISTEP (entropy: 0.15)"
- "Formula: inspect_reply (reason: evidence required)"
- "Stopped: repeated_same_command after 8 iterations"
- "Compacted context: removed 3 old messages"

This violates:
- **Rule 6**: transcript-native operational visibility
- **Rule 5**: grounded answers only — if the user can't see the reasoning, they can't verify it
- **Rule 11**: eliminate falsehoods and stalls before improving robustness — hidden stop reasons hide stalls

## Root Cause

The UI was designed for a conversational experience, not an observable one. `trace()` was added for debugging, but it was never elevated to the transcript. The status bar shows tokens and time, but not decisions.

## Proposed Solution

### Phase 1 — Define operational event types

In `src/ui_runtime_event.rs` (Task 635), add event variants for operational visibility:

```rust
pub enum UiRuntimeEvent {
    // ... existing events ...

    // Operational visibility events (collapsible transcript rows)
    PlanningDecision { source: String, confidence: f64, reason: String },
    FormulaSelected { formula: String, reason: String },
    ComplexityAssessed { complexity: String, max_iterations: usize },
    BudgetUpdated { iterations_used: usize, iterations_max: usize, tokens_used: u64 },
    CompactionTriggered { reason: String, messages_removed: usize, tokens_saved: u64 },
    StopReason { reason: String, summary: String, next_step_hint: String },
    ApproachForked { failed_approach: String, new_approach: String, reason: String },
    EvidenceCoverage { files_read: usize, coverage_threshold: usize, adequate: bool },
}
```

### Phase 2 — Emit events from decision points

1. **Planning/Complexity/Formula**: in `app_chat_loop.rs` (or `app_chat_loop/planning.rs` after Task 754), emit planning, formula, and complexity decisions after classification. Do not recreate dead routing rows if Task 751 removes routing.
2. **Budget**: in `tool_loop.rs`, emit `BudgetUpdated` after each iteration
3. **Compaction**: in `auto_compact.rs`, emit `CompactionTriggered` before and after destructive compaction. This should align with Task 737.
4. **Stop**: in `stop_policy.rs`, emit `StopReason` when the loop stops
5. **Approach fork**: in `approach_engine.rs`, emit `ApproachForked` when creating a sibling approach

### Phase 3 — Render events as collapsible transcript rows

In `src/claude_ui/claude_render.rs` or `src/ui_terminal.rs`:

1. Add a new message type: `ClaudeMessage::OperationalEvent { event, collapsed }`
2. Render operational events with dimmed styling (Grey / `fg_dim`)
3. Default to collapsed (single line: "▶ Stopped: repeated_same_command")
4. Expand to show details: "▾ Stopped: repeated_same_command\n  Iterations: 8/12\n  Last tool: read filePath=src/main.rs\n  Hint: Try rephrasing your request"
5. Use the tokenized color system (Rule: Theme)

### Phase 4 — Ensure trace and transcript alignment

1. Every `trace()` call that records a decision should also emit a `UiRuntimeEvent`
2. Every `UiRuntimeEvent` should also be written to the session trace file
3. The transcript is the primary source of truth; the trace file is a backup

## Acceptance Criteria

- [ ] `UiRuntimeEvent` includes all operational visibility variants
- [ ] Planning decision is visible in transcript (collapsed by default)
- [ ] Formula selection is visible in transcript
- [ ] Complexity assessment is visible in transcript
- [ ] Stop reason is visible in transcript with actionable hint
- [ ] Compaction trigger is visible in transcript
- [ ] Budget updates are visible in transcript (not just status bar)
- [ ] All operational events are also written to trace file
- [ ] Events use `fg_dim` styling (not primary/error colors)
- [ ] Events are collapsible/expandable
- [ ] `cargo build && cargo test` passes
- [ ] UI snapshot tests (Task 639) include operational event fixtures

## Verification Plan

- UI test: simulate planning decision → transcript contains collapsible row
- UI test: simulate stop reason → transcript shows "Stopped: ..." with hint
- UI test: simulate compaction → transcript shows "Compacted: ..."
- Integration test: run a full turn → transcript has ≥ 3 operational events
- Regression test: operational events do not clutter the main conversation flow
- Visual test: operational events are visually distinct (dimmed) from user/assistant messages

## Dependencies

- Task 635 (`ui_runtime_event.rs` — canonical event enum)
- Task 639 (`ui_snapshot.rs` — UI regression harness)
- `src/stop_policy.rs` (stop reason emission)
- `src/tool_loop.rs` (budget emission)
- `src/auto_compact.rs` (compaction emission)
- `src/claude_ui/` (rendering)

## Notes

This task is about **visibility**, not changing decisions. The system already makes these decisions; they are just hidden.

The status bar footer should remain limited to "model name, token count, elapsed time" (AGENTS.md Rule 5). Operational details belong in the transcript, not the footer.

Do not emit operational events for every internal function call. Only emit for:
- Major pipeline stage transitions (planning -> execution -> finalization)
- Budget/compaction thresholds
- Stop decisions
- Approach forks

Each event should be ≤ 3 lines when expanded. Do not dump raw JSON or tool output.

Task 739 covers the separate requirement to persist the exact left chat pane render. This task covers transcript-native operational rows.
