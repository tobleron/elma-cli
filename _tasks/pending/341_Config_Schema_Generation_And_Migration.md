# Task 341: Config Schema Generation And Migration

**Status:** pending
**Source patterns:** Opencode schema generation, Qwen-code settings schema, Roo-Code migrations
**Depends on:** Task 219 (serde path errors, completed)
**Reframed by:** Task 383 (TOML config and local data format boundary)

## Summary

Make Elma configuration TOML-first, add precise validation, and provide versioned migrations for config/session settings that change shape over time.

## Why

Reference agents reduce config breakage with schemas and migrations. Elma has many runtime and model settings, but users currently rely on runtime errors and documentation to discover config drift.

This task must not preserve JSON as a user-facing configuration goal. JSON remains acceptable for provider wire payloads or local state only when Task 383 documents that boundary.

## Implementation Plan

1. Define the canonical user-facing config format as TOML.
2. Add TOML config validation with precise path-aware errors and remediation hints.
3. Add a machine-readable config reference artifact only if it helps editor integration; do not make JSON Schema the primary contract.
4. Add a migration framework with explicit from-version and to-version functions.
5. Validate config at startup before behavior changes are applied.
6. Add tests that old TOML fixture configs migrate to the current version.

## Success Criteria

- [ ] TOML is documented and tested as the canonical config format.
- [ ] A config reference artifact can be generated reproducibly if one is added.
- [ ] Invalid config reports a field path and remediation hint.
- [ ] At least one historical config fixture migrates successfully.
- [ ] Migration failures are visible and do not corrupt the original config.
- [ ] `cargo build` and targeted config tests pass.

## Anti-Patterns To Avoid

- Do not hand-maintain schema copies that can drift.
- Do not auto-rewrite user config without backup.
- Do not mix model tuning changes with schema migration work.
- Do not use JSON config as a workaround for model-output DSL migration.
