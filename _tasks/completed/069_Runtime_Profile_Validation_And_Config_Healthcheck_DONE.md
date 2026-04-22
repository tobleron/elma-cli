# Task 069: Runtime Profile Validation And Config Healthcheck

## Priority
**P1 - RELIABILITY CORE (Tier A)**

## Objective
Add startup validation for profiles, prompts, grammars, and related config so Elma fails early and clearly when the runtime configuration is inconsistent.

## Why This Exists
Recent CLI issues showed that config shape mismatches and profile drift can break startup or create hidden runtime instability. A premium-quality local product needs immediate, explainable config validation.

## Scope
- Validate profile schema compatibility at startup.
- Detect non-profile TOML files being loaded as profiles.
- Validate grammar mappings against live profile expectations.
- Validate managed prompt sync coverage.
- Report actionable startup diagnostics instead of late failures.
- Persist a startup health summary in the session when useful.

## Status
**DONE** — Implemented and verified

## Progress Notes
- Created `src/config_healthcheck.rs` module with full validation pipeline
- Validates all 44 loaded profiles for: temperature range, top_p range, max_tokens, repeat_penalty, system_prompt non-empty
- Validates global.toml base_url is parseable
- Validates grammar file references in grammar_mapping.toml exist on disk
- Validates cross-profile consistency (base_url agreement)
- Integrated into `app_bootstrap_core.rs` — runs after `sync_and_upgrade_profiles`
- Errors halt startup with clear diagnostics; warnings print but continue
- 8 unit tests added, all passing
- `cargo build` clean, `cargo test` 220 passed, `cargo fmt` clean
