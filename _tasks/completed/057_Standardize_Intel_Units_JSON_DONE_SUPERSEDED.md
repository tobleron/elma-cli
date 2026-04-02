# 057 Standardize Intel Units JSON

## Status
**EVALUATED - SUPERSEDED / NOT TO IMPLEMENT**

## Objective
Batch update all `post_flight` implementations in `src/intel_units.rs` to be compatible with the new standardized JSON format.

## Evaluation Summary

This task is no longer aligned with the current Elma architecture.

### Why It Is Superseded

1. `src/intel_units.rs` now contains multiple **typed domain units** with intentionally different output contracts:
   - `ComplexityAssessmentUnit` expects `complexity` and `risk`
   - `WorkflowPlannerUnit` expects workflow-planning fields
   - `ScopeBuilderUnit` expects scope fields
   - `FormulaSelectorUnit` expects formula-selection fields
   - `JsonRepairUnit` expects repaired JSON text

2. The runtime now has **schema-aware parsing and repair**:
   - `src/json_error_handler.rs` defines structured schema metadata and validators
   - `src/json_parser.rs` validates typed outputs and applies repair/fallback logic
   - `src/ui_chat.rs` routes typed JSON parsing through the centralized repair pipeline

3. Forcing all intel units to accept the classifier-style envelope
   `{"choice","label","reason","entropy"}`
   would weaken validation for specialized units and blur domain contracts.

### Correct Replacement Principle

Intel units should:
- keep **specialized typed outputs** when the unit represents a domain-specific decision
- use the standardized classifier JSON only for actual classifier-style units
- rely on centralized parsing, schema validation, and repair rather than weakening `post_flight`

### Final Decision

Do **not** implement the blanket changes proposed in this task.

Task 057 is superseded by the current typed-output + schema-validation architecture already implemented in:
- `src/json_error_handler.rs`
- `src/json_parser.rs`
- `src/ui_chat.rs`
- `src/intel_units.rs`

## Requirements
1. New standardized JSON format: `{"choice": "<NUMBER>", "label": "<LABEL>", "reason": "<REASON>", "entropy": <FLOAT>}`.
2. Update `post_flight` logic for all units to:
    - Check if `output.get("label")` or `output.get("choice")` is present.
    - Only check for specialized fields if they are strictly required for that specific unit's logic.
    - Be lenient enough to allow the new standardized output to pass.
3. Specifically fix `WorkflowPlannerUnit::post_flight`.
4. Check and update:
    - `ComplexityAssessmentUnit`
    - `EvidenceNeedsUnit`
    - `ActionNeedsUnit`
    - and any other units in `src/intel_units.rs`.

## Verification
- Run `cargo build` to ensure no compile errors.
- Run `cargo test` to ensure no regressions.
- Run `./run_intention_scenarios.sh` if applicable.

## Resolution

- Reviewed against the current codebase
- Determined to be architecturally stale
- Closed without implementation
