# Task 475: Release Risk And Security Audit Gate

**Status:** pending
**Source patterns:** Goose release risk scanner, Opencode hidden-character checks, Hermes OSV/security tooling
**Depends on:** completed Task 325 (shell hardening), completed Task 339 (tool metadata policy)

## Summary

Add a local release-risk audit that flags changes touching high-risk areas such as permission gates, shell execution, provider streaming, session persistence, prompt core, tool registry, and filesystem writes.

## Why

Elma's reliability depends on a few sensitive modules. Reference agents use targeted release checks to prevent regressions in security and runtime behavior. This task formalizes that practice for Elma.

## Implementation Plan

1. Add a script or cargo subcommand that inspects changed files and classifies release risk.
   - `cargo clippy --all-targets` must pass as part of the release gate (mandatory per Task 437).
2. Add checks for hidden Unicode/control characters in source and task files.
3. Add optional dependency/security audit integration where available.
4. Require explicit warnings when `src/prompt_core.rs`, shell, permission, provider, or session schema files change.
5. Document the required checks for release candidates.

## Success Criteria

- [ ] Risk scanner reports touched high-risk modules.
- [ ] Hidden/control character scan works offline.
- [ ] Prompt-core changes are clearly called out for explicit approval.
- [ ] CI or local release checklist can invoke the gate.
- [ ] Tests or fixtures cover risk classification.

## Anti-Patterns To Avoid

- Do not make the scanner depend on network access by default.
- Do not auto-block ordinary development without clear remediation.
- Do not weaken the prompt-core protection rule.
