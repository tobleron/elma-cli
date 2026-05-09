# Task 707: Auxiliary Helper Disabled UI And Trace Contract

## Type

UI Runtime / Reasoning Visibility

## Severity

Medium

## User Requirement

When the auxiliary helper LLM is disabled or undefined, the right-side reasoning area should show the normal thinking streams, time out or disappear as it already does, and not show helper-generated summaries. This state should be explicit and quiet.

## Problem

This round did not show helper summaries, but the disabled-helper state is not yet clearly covered by a regression contract. Without a test, future changes can accidentally show summary placeholders, timeout noise, or misleading helper status when no auxiliary model is configured.

## Proposed Solution

Add an explicit disabled-helper contract.

Likely source areas:

- `src/tool_loop.rs`
- `src/intel_units/intel_units_thought_summary.rs`
- `src/reasoning_visibility.rs`
- `src/ui/ui_terminal.rs`
- `src/ui_view_state.rs`
- `src/ui_reducer.rs`

Requirements:

- When auxiliary helper config is absent or disabled, skip helper summary requests immediately.
- Do not show summary placeholder rows or helper timeout rows.
- Keep thinking streams visible according to `ReasoningVisibilityPolicy`.
- Trace one concise event such as `auxiliary_helper_disabled`.
- Add UI/reducer tests for disabled helper state.

## Acceptance Criteria

- [ ] No helper summary request is made when auxiliary config is absent.
- [ ] No helper summary placeholder or timeout noise appears in the UI.
- [ ] Thinking stream display remains unchanged.
- [ ] Trace contains a concise disabled-helper event.

## Verification Plan

Run a prompt with auxiliary helper disabled and inspect UI transcript/session trace. Add unit tests for reducer/state behavior.

