# Task 006: Extract And Expose Reasoning Audit Trail

## Objective
Capture and expose `reasoning_content` for audit, tuning, and UI visibility without making Elma's core control logic depend on it.

## Context
Reasoning traces are valuable for:
- debugging weak workflows
- tuning and calibration analysis
- user/operator visibility
- comparing model behavior across profile changes

But they are not stable enough across all local models to become a required runtime dependency for decision-making.

## Work Items
- [ ] Update the response and step data models so `reasoning_content` can be stored when present.
- [ ] In `src/models_api.rs`, ensure reasoning tokens are isolated correctly when `reasoning_format` is active.
- [ ] Expose reasoning traces in session logs for intel-unit calls.
- [ ] Improve UI display so thinking traces can be surfaced when desired, including structured-output turns where reasoning is currently hidden.
- [ ] Keep reasoning capture optional:
  - the system must still work correctly when a model emits no reasoning content
  - missing reasoning must not break orchestration or verification
- [ ] Ensure tune/calibration artifacts can inspect reasoning traces without requiring them.

## Acceptance Criteria
- Reasoning traces are stored consistently when the model provides them.
- They are available for audit and debugging in session artifacts/logs.
- UI exposure works for both plain-text and structured-output intel steps.
- Elma's core workflow logic still functions correctly with models that emit no reasoning content.

## Verification
- `cargo build`
- `cargo test`
- run a multi-step scenario and confirm reasoning content is present in session logs when available
- verify behavior remains correct with reasoning disabled or absent
