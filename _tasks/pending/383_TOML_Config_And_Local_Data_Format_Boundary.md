# Task 383: TOML Config And Local Data Format Boundary

**Status:** pending
**Priority:** high
**Suite:** Compact DSL Model-Output Migration
**Depends on:** Task 376
**Reframes:** Task 341

## Objective

Clarify and enforce Elma's non-model data format policy: user-facing configuration is TOML-first, model-produced structured output is DSL, provider wire formats may remain JSON, and local state remains whatever is safest and most compatible unless explicitly migrated.

## Required Deliverables

- Updated Task 341 scope or implementation notes.
- `docs/dsl/DATA_FORMAT_BOUNDARIES.md`
- TOML-first config schema/migration plan.
- Static checks or docs that prevent new model-output JSON prompt contracts.

## Format Policy

- Model-produced structured output: compact DSL only.
- User-facing config: TOML only unless a dependency forces another format.
- Provider request/response wire: JSON where required by API.
- Session/event storage: SQLite/JSONL/TOML allowed by task-specific tradeoff; do not churn solely for aesthetics.
- Generated reports: Markdown/TOML preferred for human and Rust-native local use.
- Test fixtures: use the format that exercises the target boundary.

## Implementation Steps

1. Update configuration documentation to distinguish config TOML from provider-wire JSON.
2. Adapt Task 341 away from "derive JSON Schema first" toward TOML validation, migration, and docs.
3. Identify any local app-owned JSON files that are user-visible config and should migrate to TOML.
4. Keep session/cache JSON if changing it would add compatibility risk without reducing model-output burden.
5. Add guardrails so new model-output prompts cannot ask for JSON without an explicit exception.

## Verification

Required commands:

```bash
rg -n "JSON Schema|Return ONLY.*JSON|valid JSON|model.*JSON" docs _tasks/pending src config
cargo fmt --check
cargo test config_healthcheck
cargo check --all-targets
```

Required checks:

- Task 341 no longer mandates JSON Schema as the primary user-facing config artifact.
- JSON boundaries are documented and intentional.
- No local config migration corrupts existing user config.

## Done Criteria

- Future contributors know when JSON is acceptable and when it is not.
- Config migration supports the DSL philosophy without unnecessary local-state churn.
- The repository stops conflating model-output JSON with all JSON serialization.

## Anti-Patterns

- Do not migrate provider HTTP payloads away from JSON.
- Do not rewrite stable session formats without a clear reliability payoff.
- Do not use TOML/RON as the model-output replacement when a compact DSL is better.
