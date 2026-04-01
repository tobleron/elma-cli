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
