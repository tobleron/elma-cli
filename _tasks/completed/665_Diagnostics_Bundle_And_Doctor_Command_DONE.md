# Task 665: Diagnostics Bundle And Doctor Command

**Status:** pending
**Priority:** HIGH
**Type:** Observability / Tooling
**Scope:** `src/diagnostics.rs`, `src/config_cmd.rs`, `src/session_*`, `src/llm_provider.rs`
**Source:** deferred task 474, user request for enterprise-grade behavior

## Summary

Add a local doctor command and diagnostics bundle that can explain environment, model endpoint, config, sessions, tool registry, and recent failure state without network dependency.

## Evidence And Gap

- Config health checks and diagnostics modules exist, but there is no comprehensive support bundle workflow.
- Session forensics currently requires manually inspecting many files.
- Cross-platform reliability requires clear environment diagnostics.

## Implementation Plan

1. Add `elma-cli config doctor` or top-level `doctor` command.
2. Validate config roots, model endpoint reachability, profile schema, grammar mappings, tool registry parity, workspace policy, terminal capabilities, and session store health.
3. Create a redacted diagnostics bundle under `sessions/<id>/diagnostics/` or `_testing_reports/`.
4. Include privacy/redaction rules for paths, env vars, headers, and model responses.

## Acceptance Criteria

- [ ] Doctor output is local-first and does not require internet.
- [ ] Sensitive values are redacted.
- [ ] Recent session failures are summarized with artifact links.
- [ ] The command exits nonzero on critical health failures.

## Verification Plan

Run doctor against a valid config and deliberately broken model/config fixtures.

