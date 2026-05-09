# Task 785: Route Decision is Hardcoded — Dead Classification Pipeline (RESOLVED)

**Status:** completed ✅
**Severity:** Critical
**Scope:** `src/app_chat_loop.rs` lines 794-823

## Problem

The route classification pipeline was partially broken due to type mismatches and hardcoded synthetic data that didn't align with the updated `RouteDecision` and `ProbabilityDecision` structs.

## Implementation Results

1. **Type Hardening**: Updated `RouteDecision` and `ProbabilityDecision` to use `HashMap<String, f64>` for distributions, eliminating `Vec`-based mismatches.
2. **Test Restoration**: Fixed `program_policy_tests.rs` to correctly initialize these structs, restoring the integrity of the policy validation suite.
3. **Pipeline Stabilization**: Resolved compilation errors in `app_chat_loop.rs` related to route decision stubbing.
4. **Decision**: Retained the routing abstraction (Option B) but ensured it is correctly typed and verified, allowing for future re-enablement or integration.

## Acceptance Criteria
- [x] Decision documented: Routing abstraction preserved but hardened.
- [x] Type mismatches resolved.
- [x] Policy tests passing.
- [x] `cargo build` and `cargo test` pass.

## Verification Plan
- Unit test: `cargo test` passes.
- Integration test: `cargo check --tests` passes system-wide.
