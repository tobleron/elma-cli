# Task 302: Semantic Continuity Tracking

## Backlog Reconciliation (2026-05-02)

Superseded by completed Task 380. If reopened, use it only as background for Task 453 request-pattern migration or certification coverage, not as a parallel implementation task.


**Status:** Pending
**References:** Directive 003

## Objective

Implement a `ContinuityTracker` that preserves the user's original intent through every pipeline transformation (routing → formula selection → execution → final answer) and verifies semantic alignment at key checkpoints.

## Scope

1. Add `ContinuityTracker` struct to `types_core.rs` with fields for original intent, selected route, formula, and alignment score
2. Wire into `app_chat_orchestrator.rs` for pre-execution alignment check
3. Wire into `tool_loop.rs` for post-execution alignment check
4. Surface continuity results as collapsible transcript rows
5. Write unit and integration tests

## Verification

```bash
cargo build
cargo test continuity
```
