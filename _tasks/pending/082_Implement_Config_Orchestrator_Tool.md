# Task 082: Implement Config Orchestrator Tool

## Continuation Checklist
- [ ] Re-read this task and all linked source/task references before editing.
- [ ] Confirm the task is still valid against current `_tasks/TASKS.md`, `AGENTS.md`, and active master plans.
- [ ] Move or keep this task in `_tasks/active/` before implementation work begins.
- [ ] Inspect the current code/config/docs touched by this task and note any drift from the written plan.
- [ ] Implement the smallest coherent change set that satisfies the next unchecked item.
- [ ] Add or update focused tests, probes, fixtures, or snapshots for the changed behavior.
- [ ] Run `cargo fmt --check` and fix formatting issues.
- [ ] Run `cargo build` and resolve all build errors or warnings introduced by this task.
- [ ] Run targeted `cargo test` commands and any task-specific probes listed below.
- [ ] Run real CLI or pseudo-terminal verification for any user-facing behavior.
- [ ] Record completed work, verification output, and remaining gaps in this task before stopping.
- [ ] Ask for sign-off before moving this task to `_tasks/completed/`.

## Priority
**P1 - EFFICIENCY & OBSERVABILITY (Tier B)**
**Depends on:** Tier A stability (tasks 065-069)

## Objective
Create a helper utility (either as a sub-command or a standalone script) to manage, validate, and visualize the complex hierarchy of TOML configuration files in `config/`.

## Context
`elma-cli` uses a highly granular configuration system with dozens of files per model/profile. As the project grows, maintaining consistency across these files (e.g., ensuring `max_tokens` or `temperature` are tuned correctly for each specific task) becomes challenging.

## Technical Details
- **Requirements**:
    - **Validation**: Check for missing required fields in TOML files.
    - **Comparison**: Compare two profiles (e.g., `balanced.toml` vs `gemma3_12b_it.toml`) to highlight differences.
    - **Visualization**: List all active profiles and their associated model/system prompts.
    - **Schema Enforcement**: Ensure all profiles adhere to the internal `Profile` struct.
- **Implementation Strategy**:
    - Consider adding a `config` sub-command to the main `elma` binary.
    - Leverage existing `Profile` loading logic in `src/app_bootstrap_profiles.rs`.

## Verification
- Run the tool against the current `config/` directory.
- It should successfully identify any malformed or inconsistent profile definitions.
