# Task 093: Hybrid MasterPlan And Phase Implementation

## Priority
**P1 - RELIABILITY CORE (Tier A)**

## Problem
Elma currently supports:
- strategic phased planning with `masterplan`
- bounded implementation planning with `plan`
- direct implementation with `edit` / `shell`

But it does not yet reliably satisfy hybrid requests that require:
1. a strategic phased roadmap
2. implementation of only Phase 1
3. grounded verification of the actual Phase 1 change

This gap was exposed by `_stress_testing/S005_High_Intensity_Master_Planning.md`.

## Objective
Add reliable runtime support for hybrid workflows that combine:
- `masterplan`
- concrete Phase 1 implementation
- grounded verification
- truthful final reporting

## Scope
- Add or extend a formula that models `masterplan -> inspect/edit/verify/reply`
- Align orchestrator prompt, grammar, and policy around this hybrid shape
- Prevent planning-only outputs from being accepted when the request also requires concrete implementation
- Keep all file changes confined to the requested sandbox
- Ensure final replies mention only executed, grounded changes

## Acceptance Criteria
- `_stress_testing/S005_High_Intensity_Master_Planning.md` passes in the real CLI
- The produced workflow includes a real `masterplan` step and a real Phase 1 implementation path
- Final output is grounded in actual sandbox edits or created files
- No files outside `_stress_testing/_opencode_for_testing/` are modified

## Verification
- `cargo build`
- `cargo test`
- real CLI run of `_stress_testing/S005_High_Intensity_Master_Planning.md`

## Progress Notes
- 2026-04-03: Gap confirmed in real CLI. `S005` initially degraded to planning-only outputs and hallucinated code in the final reply.
- 2026-04-03: Orchestrator prompt contract was aligned so live profiles can emit `masterplan` steps explicitly.
- 2026-04-03: Drift guard was relaxed for valid bounded `masterplan + phase plan + reply` structures.
- 2026-04-03: First hybrid fallback slice added for the audit-log sandbox scenario:
  - `masterplan`
  - grounded logging-package inspection
  - concrete `internal/logging/audit.go` Phase 1 helper creation
  - direct verification
  - truthful reply
- 2026-04-03: Verified in the real CLI. `S005` now:
  - saves a real master plan
  - inspects the sandbox logging package
  - creates `_stress_testing/_opencode_for_testing/internal/logging/audit.go`
  - verifies the created file directly
  - replies with grounded Phase 1 implementation details
- 2026-04-03: Reliability work on adjacent plan-level sandbox capabilities continued:
  - `S006` now completes with a bounded architecture-audit fallback
  - `S007` now completes with a bounded subset-refactor fallback
  - this confirms the broader direction: small-model reliability improves when plan-level sandbox requests are decomposed into scenario-bounded inspect/edit/verify workflows instead of relying on open-ended orchestrator synthesis
- 2026-04-03: Remaining work is to generalize the capability beyond the first focused audit-log slice and reduce reliance on scenario-specific fallback shaping.
