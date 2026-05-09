# Task 674: Cargo Dependency Feature Hygiene And Supply Risk Audit

**Status:** pending
**Priority:** MEDIUM
**Type:** Security / Maintenance
**Scope:** `Cargo.toml`, `Cargo.lock`, `elma-tools/Cargo.toml`, feature flags, `_scripts/`
**Source:** deferred task 477

## Summary

Audit dependencies, features, duplicate crates, default features, unused dependencies, and supply-chain risk without bloating Elma.

## Evidence And Gap

- The project has grown many reliability, UI, session, and document dependencies.
- Completed tasks added several crates; the current feature set needs a hygiene pass.
- Offline-first users benefit from smaller, more predictable builds.

## Implementation Plan

1. Run dependency tree and unused dependency analysis.
2. Identify duplicated crates, large optional features, default features that can be disabled, and security advisories.
3. Keep any removal/change behind focused tests and compile checks.
4. Document intentional heavyweight dependencies.

## Acceptance Criteria

- [ ] Dependency changes preserve existing behavior.
- [ ] Optional/network-heavy features are off by default where practical.
- [ ] Supply risks are documented with mitigation.
- [ ] Build and tests pass after cleanup.

## Verification Plan

Run `cargo check --all-targets`, `cargo tree`, audit tooling if installed, and targeted tests for changed surfaces.

