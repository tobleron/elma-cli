# Task 771: Remove Word-Based Routing from Complexity Assessor

## Type
Architecture / Refactor

## Severity
Critical

## Scope
System-wide / Module

## Problem
`src/complexity_assessor.rs` implements task complexity assessment using hardcoded word triggers (e.g., `multi_edit_signals`, `code_change_signals`). This is a direct violation of Architectural Rule 1 ("No Word-Based Routing").

```rust
    let multi_edit_signals = [
        "all docs",
        "every file",
        "all files",
        ...
    ];
    let has_multi_signal = multi_edit_signals.iter().any(|s| lower.contains(s));
```

## Root Cause
The complexity assessor was originally implemented as a fast-path heuristic to save model tokens, but it has become a brittle decision point that contradicts the project's core philosophy of model-based reasoning.

## Proposed Solution
Replace the heuristic-based `assess_complexity` with an Intel Unit call that uses the model to classify complexity.

- Phase 1: Update `src/complexity_assessor.rs` to use the `complexity_assessor` intel unit (from `config/defaults/complexity_assessor.toml`).
- Phase 2: Remove the `multi_edit_signals`, `code_change_signals`, and `investigate_signals` arrays.
- Phase 3: Preserve a minimal "fast-path" only for extremely simple greetings (e.g., "hi", "thanks") as allowed by Rule 2, but ensure all other requests go through the model.
- Phase 4: Update unit tests to verify model-based classification (using mocks if necessary).

## Acceptance Criteria
- [ ] No hardcoded keyword lists are used for routing or complexity decisions in `src/complexity_assessor.rs`.
- [ ] Complexity assessment is performed by the `complexity_assessor` intel unit for all non-trivial requests.
- [ ] The system correctly distinguishes between DIRECT and MULTISTEP complexity using model reasoning.

## Verification Plan
- Unit test: `cargo test complexity_assessor`
- Integration test: Run real CLI and verify (via `--trace`) that complex requests are correctly identified as MULTISTEP without keyword matching.

## Dependencies
None

## Notes
- Align with the new two-gate complexity system (DIRECT/MULTISTEP) established in Task 766.
- Ensure the `max_iterations` scaling remains correct for each level.
