# Task 650: Runtime Keyword Gate Audit And Analyzer Rule

**Status:** pending
**Priority:** HIGH
**Type:** Test Coverage / Architecture
**Scope:** `src/app_chat_patterns.rs`, `src/complexity_assessor.rs`, `src/guardrails.rs`, `src/program_policy.rs`, `_scripts/run_analyzer.sh`
**Source:** AGENTS.md no keyword matcher rule; `_knowledge_base` audit recommendation

## Summary

Audit runtime keyword or substring gates and add analyzer coverage that distinguishes unsafe deterministic parsing from forbidden intent/routing classification.

## Evidence And Gap

- `src/program_policy.rs`, `src/shell_preflight.rs`, and `src/complexity_assessor.rs` contain many string/substring checks.
- Some checks are legitimate structural safety checks; routing/classification word triggers are not.
- The repo needs a repeatable guard so future contributions do not reintroduce keyword matchers.

## Implementation Plan

1. Inventory `contains`, regex, and literal pattern checks in routing, complexity, guardrails, shell policy, and UI command parsing.
2. Classify each as allowed structural validation, command syntax parsing, UI command registry, or forbidden intent classification.
3. Replace forbidden runtime gates with model-informed strict JSON decisions or typed deterministic state.
4. Add analyzer checks with allowlist comments for legitimate validators.

## Acceptance Criteria

- [ ] No user-intent route/complexity decision depends on raw keyword matching.
- [ ] Shell/path safety parsing remains deterministic and explainable.
- [ ] Analyzer output flags new forbidden gates with file/function context.
- [ ] Documentation explains the distinction.

## Verification Plan

Run `_scripts/run_analyzer.sh`, `cargo test routing complexity`, and spot-check `rg "contains\\(" src`.

