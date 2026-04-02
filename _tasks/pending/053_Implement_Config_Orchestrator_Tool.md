# ⏸️ POSTPONED

**Status:** POSTPONED until P0-1, P0-2, P0-3, P0-4 complete

**Reason:** Per REPRIORITIZED_ROADMAP.md, these advanced features are blocked until the 4 foundational pillars are stable:
- P0-1: JSON Reliability (Tasks 001-004)
- P0-2: Context Narrative (Tasks 005-007)
- P0-3: Workflow Sequence (Tasks 008-011)
- P0-4: Reliability Tasks (Tasks 012-018)

**Do not start work on this task** until all P0-1 through P0-4 tasks are complete.

---

# Task 053: Implement Config Orchestrator Tool

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
