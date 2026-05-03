# Task 367: Core DSL Actions E2E Self-Test Suite

**Status:** pending
**Priority:** high
**Suite:** DSL Protocol And Skills Certification
**Depends on:** Task 365 (DSL protocol self-test harness), Task 366 (DSL action contract tests), Task 378 (action DSL cutover)

## Objective

Certify the core non-mutating DSL actions end to end: `R`, `L`, `S`, `Y`, restricted `X`, `ASK`, and `DONE`.

## Required Deliverables

- prompt scenarios under `tests/dsl/prompts/core_actions.md`
- automated assertions where possible
- transcript examples linked from `docs/dsl/SELF_TEST_PROMPTS.md`
- regression tests for any failures discovered

## Built-In Elma CLI Prompt Pack

```text
Search the sandbox for DSL_CORE_ALPHA. Read the matching file. Answer with the exact file path, line number, and surrounding sentence. Do not use `X` unless search/read cannot solve it.
```

```text
List the sandbox root, then read the file named inspection_fixture.txt and report its first three lines.
```

```text
Use an allowed verification command to run `git status` in the sandbox fixture repository, then explain why the command was allowed.
```

```text
Search symbols for DSL_SYMBOL_ALPHA, read the containing file, and report the symbol location.
```

```text
Ask a clarifying question only if the requested file path is ambiguous; otherwise inspect and finish with DONE.
```

```text
Finish only after evidence proves what file contains DSL_CORE_BETA.
```

## Expected Tool Behavior

- `S` locates sentinel strings without `X` fallback.
- `R` returns exact file content and headers.
- `L` lists only validated workspace-relative paths.
- `Y` returns bounded symbol-oriented results.
- `X` goes through strict allowlist policy and permission where appropriate.
- `ASK` stops the loop with a user question.
- `DONE` is the only successful finalization command.

## Verification

Required commands:

```bash
cargo fmt --check
cargo test agent_protocol
cargo test tool_loop
cargo test stop_policy
cargo test session_flush
cargo test evidence_ledger
cargo build
```

Prompt pass criteria:

- each prompt uses the expected tool family
- final answers are supported by evidence
- transcript contains tool start/result rows
- session artifacts contain flushed tool results
- no raw DSL leaks as assistant prose
- no unsupported action/tool is advertised

## Done Criteria

- All core DSL actions pass prompt tests in the harness.
- Any failure has a regression test.
- Results are recorded in the DSL protocol matrix.
