# Task 449: Cargo Dependency And Feature Hygiene Audit

**Status:** pending
**Priority:** MEDIUM
**Source:** 2026-05-02 full codebase audit
**Related:** pending Task 414

## Summary

Audit dependencies, feature flags, dev dependencies, and manifest hygiene for dead weight, security exposure, and build cost.

## Evidence From Audit

- `Cargo.toml` includes many document, UI, terminal, archive, and provider dependencies after several rollback/rebaseline tasks.
- `serde_yaml` appears as a dev dependency at `0.9`, which Cargo reports as deprecated in the package name.
- `djvu-rs`, `mobi`, `epub`, `zip`, and related document dependencies may not all be wired to real extraction paths.
- `fetch` is declared in tool metadata while network execution is still pending, so network-related dependencies/features need policy review.
- `Cargo.toml` has minor manifest hygiene issues, such as an extra leading space before `flate2`.

## User Decision Gate

Ask the user which dependency policy they prefer:

- Keep optional format dependencies for near-term planned adapters.
- Remove unused dependencies until their tasks are active.
- Gate heavier/optional dependencies behind Cargo features.

## Implementation Plan

1. Run a local dependency usage audit using source search and, if approved/available, `cargo machete` or `cargo udeps`.
2. Map each suspicious dependency to an active/pending task.
3. Ask the user before removing or feature-gating dependencies tied to future work.
4. Clean obvious manifest formatting.
5. Add dependency audit notes to Task 414 release gate if approved.

## Success Criteria

- [ ] Every nontrivial dependency has an owner or pending task.
- [ ] Deprecated/dev-only dependencies have an upgrade/remove plan.
- [ ] Optional format dependencies are feature-gated or explicitly retained.
- [ ] `cargo check --all-targets` passes after changes.

## Anti-Patterns To Avoid

- Do not remove dependencies that pending tasks intentionally rely on.
- Do not require network-based audit tools by default.
- Do not trade offline-first behavior for convenience.
