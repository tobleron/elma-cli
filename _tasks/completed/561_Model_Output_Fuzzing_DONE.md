# 561 — Add Comprehensive Model-Output Fuzzing and Repair Tests

- **Priority**: High
- **Category**: Testing
- **Depends on**: 559 (defensive JSON parsing)
- **Blocks**: None

## Problem Statement

The codebase has extensive subsystems that consume model-generated text:
- Tool call argument JSON (parsed in `tool_calling.rs`)
- Intel unit output JSON (parsed in `intel_trait.rs`, `json_parser.rs`)
- Classification JSON (parsed in `routing_parse.rs`)
- Formula selection output (parsed in `formulas/`)
- DSL action output (parsed in DSL parser)

None of these have fuzzing tests with corrupted/malformed model output. The test coverage for JSON parsing is exclusively "happy path" — valid JSON produces correct output. There are no tests for:
- What happens when a small model produces a trailing comma
- What happens when a model wraps JSON in markdown code fences
- What happens when a model includes thinking/reasoning text inside JSON
- What happens when a model hallucinates non-existent fields

## Why This Matters for Small Local LLMs

This is the single highest-impact testing gap for small-model reliability. Small models produce malformed JSON at significantly higher rates than large models. Without fuzzing tests:

1. Each new model version may introduce new failure patterns
2. The JSON repair pipeline evolves without regression protection
3. "Silent failure" (falling back to defaults) hides bugs that only manifest as wrong behavior

## Current Behavior

The JSON repair pipeline (`json_repair.rs`) and intel unit parsing (`json_parser.rs`, `intel_trait.rs`) have some error handling but no systematic fuzzing. The `jsonrepair-rs` crate may or may not be wired in.

## Recommended Target Behavior

Create a comprehensive fuzzing test suite:

1. **Golden corpus**: Collect real model outputs (valid and malformed) from actual small-model runs
2. **Mutation fuzzer**: Take valid JSON and systematically corrupt it with common small-model errors
3. **Regression suite**: Every time a new malformation pattern is discovered, add it to the corpus
4. **Recovery rate tracking**: Measure what percentage of corrupted inputs are successfully repaired

### Test Categories

| Category | Description | Target |
|----------|-------------|--------|
| Syntax errors | Trailing commas, unquoted keys, single quotes, missing braces | 95% recovery |
| Structural errors | Wrong nesting, extra/missing array elements | 70% recovery |
| Semantic errors | Wrong field types, missing required fields | 50% recovery + clear error |
| Extraneous content | Markdown fences, prose wrapping, thinking tags | 90% recovery |
| Encoding issues | UTF-8 edge cases, control characters, BOM | 100% recovery |
| Boundary cases | Empty JSON, deeply nested, very long strings | No panic |

## Source Files That Need Modification

- `src/json_parser.rs` — Add test module with fuzzing
- `src/json_repair.rs` — Add test module with repair regression tests
- `src/tool_calling.rs` — Add test module with tool argument fuzzing

## New Files/Modules

- `tests/fixtures/model_outputs/` — Golden corpus directory
- `tests/fixtures/model_outputs/valid/` — Known-valid model outputs
- `tests/fixtures/model_outputs/malformed/` — Known-malformed outputs with expected repairs
- `tests/fuzzing/mod.rs` — Fuzzing harness entry point
- `tests/fuzzing/json_fuzzer.rs` — JSON mutation engine

## Step-by-Step Implementation Plan

1. Create `tests/fixtures/model_outputs/` directory structure
2. Collect real model outputs from session traces (anonymize if needed)
3. Create `JsonMutator` that applies common small-model errors:
   ```rust
   enum JsonMutation {
       TrailingComma,
       UnquotedKey,
       SingleQuote,
       MissingClosingBrace,
       ExtraClosingBrace,
       MarkdownFence,
       ProsePrefix,
       ProseSuffix,
       ThinkingTags,
       WrongNumberType,   // "3" instead of 3
       MissingRequiredField(String),
       ExtraField(String, serde_json::Value),
   }
   ```
4. Implement `mutate_json(valid: &str, mutations: &[JsonMutation]) -> String`
5. For each JSON type in the system, create a fuzzing test:
   - Generate valid instance from schema
   - Apply random mutations
   - Feed to `parse_model_json<T>()`
   - Verify: either successful parse or clear error (no panic, no infinite loop)
6. Add regression tests for known real-world failures
7. Add golden tests: save expected output for each malformed input
8. Run fuzzing as part of CI (but with reasonable time limits)

## Recommended Crates

- `proptest` — property-based testing for generating valid JSON instances
- `similar` — already a dependency; use for diff-based failure reporting
- `serde_json` — use `json!` macro and `to_value` for test fixture generation

## Validation/Sanitization Strategy

- Fuzzing tests must NEVER make network calls
- Fuzzing tests must be deterministic (fixed random seed)
- All mutations must be reversible (record what was changed)
- Test output includes diff between expected and actual

## Testing Plan

The fuzzing tests ARE the testing plan. Additionally:

1. **CI integration**: Run fuzzing on every PR (capped at 5 seconds per test)
2. **Nightly deep fuzz**: Run extended fuzzing (60 seconds per test) on nightly build
3. **Regression gate**: Any PR that reduces recovery rate below threshold fails CI
4. **Coverage report**: Track which mutation types are covered by tests

## Acceptance Criteria

- All JSON parsing entry points have fuzzing tests
- Fuzzing covers at least 6 mutation categories
- Recovery rate for syntax errors is ≥90%
- Recovery rate for structural errors is ≥60%
- No panics on any malformed input
- Golden corpus has ≥20 real model outputs
- Fuzzing runs in CI (fast mode: 5s per test)
- Regression test added for each new failure pattern discovered

## Risks and Migration Notes

- **Flaky tests**: Fuzzing with random mutations can produce flaky results. Always use fixed seeds.
- **Test data sensitivity**: Real model outputs may contain sensitive information. Create anonymized fixtures.
- **Time investment**: Comprehensive fuzzing requires ongoing maintenance. Start with the 5 most critical JSON types and expand.
- This task pairs with Task 559 — implement the repair pipeline first, then fuzz it.
