# 583 — Implement Config Validation at Startup with Clear Error Messages

- **Priority**: Medium
- **Category**: Config
- **Depends on**: None
- **Blocks**: None

## Problem Statement

Configuration is loaded from multiple sources (`config/defaults/`, `config/runtime.toml`, `config/profiles.toml`, `elma.toml`) with a fallback chain. However, config loading appears to use `unwrap_or_default()` patterns that silently swallow misconfigurations:

1. **Missing required fields**: Silently defaulted instead of reporting an error
2. **Invalid values**: Type mismatches in TOML may be silently coerced or defaulted
3. **Profile references**: A config referencing a non-existent profile may fail at runtime instead of startup
4. **Conflicting settings**: No detection of contradictory configuration

## Why This Matters for Small Local LLMs

Small models are sensitive to configuration — wrong temperature, wrong max_tokens, or wrong model ID can silently degrade performance. Users should know immediately if their config is invalid, not discover it through poor model behavior.

## Recommended Target Behavior

Add a config validation step at startup:

```rust
pub struct ConfigIssues {
    pub errors: Vec<ConfigError>,    // Must be fixed
    pub warnings: Vec<ConfigWarning>, // Advisory
}

pub enum ConfigError {
    MissingRequiredField { file: String, field: String },
    InvalidValue { file: String, field: String, value: String, expected: String },
    ProfileNotFound { reference: String, profile: String },
    InvalidPath { field: String, path: String },
    ConflictingSettings { setting1: String, setting2: String },
}

pub enum ConfigWarning {
    DefaultUsed { file: String, field: String, default: String },
    DeprecatedField { file: String, field: String, replacement: String },
    UnknownField { file: String, field: String },
}
```

Validation rules:
1. All model profile references must resolve to existing profiles
2. Numeric fields must be in valid ranges (temperature 0.0-2.0, max_tokens > 0, timeout_s > 0)
3. Path fields must exist or be creatable
4. No conflicting settings (e.g., `ctx_max < tool_loop_max_tokens_cap`)
5. At least one model profile must be defined

## Source Files That Need Modification

- `src/defaults.rs` — Add validation pass after loading
- `src/defaults_core.rs` — Add validation helpers
- `src/llm_config.rs` — Validate model configuration
- `src/config_healthcheck.rs` — Enhance existing healthcheck or replace

## New Files/Modules

- `src/config_validate.rs` — Validation rules and error types

## Acceptance Criteria

- Config validation runs at startup (before model calls)
- Errors are reported clearly to the user (file, field, problem, fix)
- Warnings are reported for non-critical issues
- Validation catches: missing profiles, invalid ranges, non-existent paths
- Config errors cause early exit with actionable message (not panic)
- `elma-cli --validate-config` command available

## Risks and Migration Notes

- Current config with minor issues may have been working silently. Adding strict validation could break existing setups. Start with warnings, escalate to errors over time.
- The `config_healthcheck.rs` file exists — enhance rather than replace.
