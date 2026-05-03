# 572 — Add Golden Tests for Tool Output Formats

- **Priority**: Medium
- **Category**: Testing
- **Depends on**: 552 (split tool_calling.rs), 565 (standardized tool result envelope)
- **Blocks**: None

## Problem Statement

Tool output formats are critical for model comprehension. When a tool's output format changes, it can confuse small models that have learned to parse specific patterns. There are no golden tests that freeze tool output formats and catch unintended changes.

## Why This Matters for Small Local LLMs

Small models are pattern-matchers — they learn to extract information from specific output formats. An unintentional format change (e.g., `read` tool output changing from `Filename:\ncontent` to `### File 1: filename\ncontent`) can cause the model to misinterpret results.

## Recommended Target Behavior

Create golden tests using `insta` (already a dev-dependency) or `similar` (already a dependency) for each tool's output format:

```rust
#[test]
fn test_read_tool_output_format() {
    let result = exec_read(...);
    insta::assert_snapshot!("read_single_file", result.model_facing_summary);
}

#[test]
fn test_shell_tool_success_format() {
    let result = exec_shell(...);
    insta::assert_snapshot!("shell_ls_output", result.model_facing_summary);
}
```

## Source Files That Need Modification

- Each tool executor file (after Task 552 split) — Add golden test modules
- `tests/fixtures/tool_outputs/` — Golden test fixtures

## New Files/Modules

- `tests/fixtures/tool_outputs/read/` — Read tool golden outputs
- `tests/fixtures/tool_outputs/shell/` — Shell tool golden outputs
- `tests/fixtures/tool_outputs/search/` — Search tool golden outputs
- (One directory per tool)

## Acceptance Criteria

- Golden tests for all 30+ tool output formats
- Tests catch unintended format changes
- Tests run in CI
- Test fixtures are versioned

## Risks and Migration Notes

- Golden tests are brittle — they break on ANY format change. Review snapshot changes carefully.
- Use `insta` review workflow (`cargo insta review`) to accept intentional changes.
