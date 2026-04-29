# Task 377: Compact DSL Family Core Parser And Error Model

**Status:** pending
**Priority:** critical
**Suite:** Compact DSL Model-Output Migration
**Depends on:** Task 376
**Blocks:** Tasks 378, 380, 381, 382, 384

## Objective

Build the shared DSL infrastructure used by every migrated model-output contract. The parser layer must treat LLM text as untrusted input: trim only outer whitespace, reject malformed output precisely, never extract a best-looking fragment, and render compact repair observations suitable for small local models.

## Required Deliverables

- `src/dsl/` or `src/agent_protocol/` module family with parser primitives, error types, sanitization, repair rendering, and tests.
- Shared error enum with stable codes such as `INVALID_DSL`, `UNSAFE_PATH`, `UNSAFE_COMMAND`, `INVALID_EDIT`, and `UNSUPPORTED_DSL`.
- A compact renderer for retry/repair feedback.
- Documentation in `docs/dsl/DSL_CONTRACTS.md`.

## Design Requirements

Use a small DSL family, not one giant language:

- action DSL for workspace/executor actions
- verdict DSLs for one-token or two-field decisions
- scope/list DSLs for bounded lists
- repair DSLs for compact correction instructions

Common parser rules:

- Reject empty output.
- Reject Markdown fences.
- Reject JSON/XML/YAML/TOML/prose when the expected output is DSL.
- Reject extra text before or after a command.
- Reject unterminated quotes and blocks.
- Reject duplicate fields.
- Do not auto-close quotes, blocks, or missing end markers.
- Do not strip inner whitespace from block bodies.
- Normalize CRLF to LF only where the grammar explicitly allows block content.

Sanitization rules:

- Strip or reject ANSI/control sequences before parsing, using the existing ANSI helper where available.
- Reject NUL bytes.
- Preserve exact user/model text in diagnostics only after truncation and control-character escaping.
- Use `regex` only for bounded lexical checks such as command names, identifiers, and safe field keys.
- Use explicit hand-written parsing for field order, quoted strings, blocks, and trailing-garbage detection.

## Implementation Steps

1. Add core types: `DslError`, `DslErrorCode`, `DslResult<T>`, `RepairObservation`, `ParseContext`.
2. Add lexical helpers for uppercase command tokens, strict quoted strings, line markers, and compact key/value fields.
3. Add sanitization helpers for ANSI/control characters and bounded debug previews.
4. Add renderer helpers that produce exact compact feedback:

```text
INVALID_DSL
error: missing ---END
return exactly one valid command
```

5. Add test-only fixtures for valid/invalid single-line, block, and list-style DSL outputs.
6. Document which modules should use the shared parser instead of ad hoc parsing.

## Verification

Required commands:

```bash
cargo fmt --check
cargo test dsl
cargo test agent_protocol
cargo check --all-targets
```

Required coverage:

- empty output rejected
- fenced output rejected
- JSON-looking output rejected
- prose before command rejected
- prose after command rejected
- duplicate field rejected
- malformed quote rejected
- missing end marker rejected
- control/NUL handling tested
- compact repair rendering stable

## Done Criteria

- Every later DSL parser can reuse the same strict primitives and error renderer.
- Invalid model output produces a short repair observation, not a panic or free-form error.
- Parser behavior is deterministic and fully covered by focused tests.

## Anti-Patterns

- Do not build a permissive extractor.
- Do not rely on regex as the whole parser.
- Do not add examples-heavy prompts as a substitute for grammar and validation.
