# Task 445: Clippy Lint Gate Baseline Cleanup

**Status:** pending
**Priority:** HIGH
**Source:** 2026-05-02 full codebase audit
**Related:** pending Task 399, pending Task 414

## Summary

Make the current `cargo clippy --all-targets` baseline green, then decide which lint gate belongs in routine verification.

## Evidence From Audit

`cargo check --all-targets` passes, but `cargo clippy --all-targets` currently fails with:

- `src/shell_preflight.rs:886`: `result.estimated_count >= 0` is always true.
- `src/shell_preflight.rs:898`: `result.estimated_count >= 0` is always true.
- `src/ui/ui_spinner.rs:171`: `elapsed.as_micros() >= 0` is always true.

These are test-quality issues, but they block Clippy.

## User Decision Gate

Ask the user which lint policy they want after the baseline is fixed:

- Clippy must pass before completing code tasks.
- Clippy is advisory except release gates.
- Only selected lint groups are enforced initially.

Update Task 399 and Task 414 expectations if the user chooses enforcement.

## Implementation Plan

1. Replace tautological assertions with meaningful assertions.
2. Run `cargo clippy --all-targets`.
3. Decide the ongoing lint policy with the user.
4. Wire the chosen policy into verification planner/release gate tasks.

## Success Criteria

- [ ] `cargo clippy --all-targets` passes locally.
- [ ] The three tautological tests assert useful behavior.
- [ ] The chosen lint policy is documented.
- [ ] Verification tasks know whether Clippy is mandatory or advisory.

## Anti-Patterns To Avoid

- Do not suppress Clippy warnings just to get green.
- Do not add expensive lint steps to every tiny edit without Task 399 policy.
- Do not ignore test-only quality issues when they block a release gate.
