# Task 025: Decouple Model-to-Profile Mapping

## Context
Currently, `sync_and_upgrade_profiles` in `src/app_bootstrap_profiles.rs` synchronizes most profiles to use the same `base_url` and `model_id`. This prevents using different models for different tasks (e.g., small models for routing, large models for planning).

## Objective
Modify the profile loading and synchronization logic to allow per-profile model overrides:
- Update `Profile` struct to allow optional model and base_url.
- Modify `sync_and_upgrade_profiles` to only override if specifically requested or if a global "sync" flag is set.
- Enable `config/profiles.toml` to specify different models for different profile names.

## Success Criteria
- System can run with multiple different local/remote models simultaneously.
- `cargo build` succeeds with zero warnings.
