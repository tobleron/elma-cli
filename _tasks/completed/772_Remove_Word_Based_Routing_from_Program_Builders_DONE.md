# Task 772: Remove Word-Based Routing from Program Builders

## Type
Architecture / Refactor

## Severity
Critical

## Scope
System-wide / Module

## Problem
`src/app_chat_builders_advanced.rs` and `src/app_chat_patterns.rs` use hardcoded word triggers (e.g., `lower.contains("function")`, `lower.contains("rename")`) to select hardcoded programs. This is a direct violation of Architectural Rule 1 ("No Word-Based Routing").

Additionally, these files contain several legacy "stress-test" specific programs and patterns that bypass the standard planning and orchestration layers.

## Root Cause
These builders were created as specialized "probes" and "stress-test" handlers during early development. They have not been integrated into the modern WorkGraph/IntelUnit-based planning system.

## Proposed Solution
Delete the word-based pattern matching and migrate any useful logic into the standard planning/formula system.

- Phase 1: Identify useful programs in `src/app_chat_builders_advanced.rs` (e.g., function call-site search) and convert them into reusable Recipes or Formula-based WorkGraph templates.
- Phase 2: Delete `src/app_chat_patterns.rs` entirely.
- Phase 3: Remove the `lower.contains` logic from `src/app_chat_builders_advanced.rs` and ensure the `Objective → WorkGraph` pipeline handles these requests via model reasoning.
- Phase 4: Clean up `src/app_chat_builders_audit.rs` and other related "builder" modules that use keyword triggers.

## Acceptance Criteria
- [ ] `src/app_chat_patterns.rs` is deleted.
- [ ] No hardcoded word triggers are used for program selection in `src/app_chat_builders_advanced.rs`.
- [ ] All user requests flow through the model-based intent/complexity/planning pipeline.
- [ ] Useful "expert" capabilities (like call-site search) are preserved as model-suggested formulas or recipes.

## Verification Plan
- Unit test: `cargo test app_chat_builders` (verify removal of old patterns).
- Integration test: Request a function search or a rename refactor and verify that the system plans the work via the WorkGraph instead of jumping to a hardcoded program.

## Dependencies
None

## Notes
- Align with "Rule 3 — Decomposition For Small Models".
- Avoid creating giant prompts in the new planning phase; keep the planner focused on graph structure.
