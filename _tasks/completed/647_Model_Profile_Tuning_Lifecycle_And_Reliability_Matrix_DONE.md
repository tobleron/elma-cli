# Task 647: Model Profile Tuning Lifecycle And Reliability Matrix

**Status:** pending
**Priority:** HIGH
**Type:** Model Robustness / Configuration
**Scope:** `config/`, `src/tune*`, `src/model_capabilities.rs`, `src/llm_config.rs`
**Source:** postponed tasks 074, 087, 090, 091, 092; user request for modern and dense model support

## Summary

Create a per-model reliability matrix that records JSON cleanliness, tool-call fidelity, reasoning behavior, streaming quality, timeout profile, context limits, and recommended runtime settings.

## Evidence And Gap

- `config/*/model_behavior.toml` stores some JSON behavior fields, but there is no comprehensive tested profile lifecycle.
- Old tuning tasks were postponed and scattered across model behavior mapping, token telemetry, tuning lifecycle, and upgrade advice.
- User specifically noted Elma may not support dense coder models well.

## Implementation Plan

1. Define a stable model reliability record stored under `config/<model>/model_reliability.json` or TOML equivalent.
2. Run offline smoke scenarios for JSON, tool args, streaming, finalization, reasoning visibility, and long context behavior.
3. Keep prompt mutation disabled; tune only numeric/provider/runtime fields.
4. Surface model risk and degraded capability notices in transcript rows.
5. Add `config` command output for reliability status and last probe date.

## Acceptance Criteria

- [ ] Every configured model has a capability/reliability record or an explicit unknown status.
- [ ] Dense coder model failures produce recommended deterministic mitigations.
- [ ] Tuning cannot rewrite prompts or core policy.
- [ ] Reliability data is used by request adapters and context budgets.

## Verification Plan

Run model probe scenarios on at least one local small model and one dense coder model, then inspect config and session diagnostics.

