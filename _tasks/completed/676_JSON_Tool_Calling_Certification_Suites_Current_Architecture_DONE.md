# Task 676: JSON Tool Calling Certification Suites Current Architecture

**Status:** pending
**Priority:** HIGH
**Type:** Test Coverage / Certification
**Scope:** `tests/`, `scenarios/`, `_testing_prompts/`, `_testing_reports/`, `src/tool_loop.rs`
**Source:** deferred task 471, deferred task 480, strict JSON architecture

## Summary

Build certification suites for the current strict JSON/tool-calling architecture using real CLI/session behavior and knowledge-base parity scenarios.

## Evidence And Gap

- Historical certification tasks had DSL terminology; the useful idea is repeatable protocol/tool/session certification.
- `_knowledge_base` contains tool and architecture patterns that can become certification scenarios.
- Elma needs confidence that all tools, traces, and finalization behave under small-model constraints.

## Implementation Plan

1. Create smoke prompts for each core tool family, model output shape, and finalization path.
2. Add session regression scans for artifacts, transcript rows, event completion, and evidence grounding.
3. Include knowledge-base parity scenarios for tool registry, shell policy, safe writes, and UI event flow.
4. Ensure suite names and assertions do not depend on DSL.

## Acceptance Criteria

- [ ] Certification suite covers current model-native strict JSON/tool calls.
- [ ] Session artifacts are part of pass/fail criteria.
- [ ] Tool declaration/executor/schema parity is checked.
- [ ] Results are written to `_testing_reports/`.

## Verification Plan

Run certification suite locally against a fake provider and at least one real local model when available.

