# Task 677: Release Risk Security Audit Gate

**Status:** pending
**Priority:** MEDIUM
**Type:** Security / Release
**Scope:** `_scripts/`, `src/permission_gate.rs`, `src/workspace_policy.rs`, `src/tool_calling.rs`, docs
**Source:** deferred task 475

## Summary

Define a release gate that checks security, privacy, path boundaries, network defaults, permission gates, and session artifact sensitivity.

## Evidence And Gap

- Many safety systems exist, but no single release checklist enforces them together.
- User wants enterprise-grade behavior that works in every condition.

## Implementation Plan

1. Create a release checklist/task script for safety-critical invariants.
2. Check network-off defaults, workspace-only tool boundaries, permission prompts, secret redaction, shell policy, and config health.
3. Require current certification and portability gates before release.
4. Document residual risks and known limitations.

## Acceptance Criteria

- [ ] Release gate fails on critical safety regressions.
- [ ] Security/privacy findings are actionable and file-scoped.
- [ ] Network-enabled features are opt-in and clearly marked.
- [ ] Gate output can be attached to release notes.

## Verification Plan

Run gate on a clean tree and with injected safety regressions.

