# Task 461: Local Code Interpreter Tool Wrappers

**Status:** Pending
**Priority:** MEDIUM
**Estimated effort:** 3-5 days
**Dependencies:** Task 387, Task 459, Task 460
**References:** source-agent parity: AgenticSeek language interpreters, Open Interpreter run/bash/edit tools

## Objective

Provide structured local code execution wrappers for common interpreters while preserving Elma's permission, timeout, sandbox, and rust-first policies.

## Scope

- Start with Python and Node if available.
- Add Rust/Cargo script execution only after sandbox boundaries are proven.
- Treat Go, Java, and C as follow-up adapters using the same framework.

## Implementation Plan

1. Add an interpreter registry with prerequisite checks.
2. Add tool declarations such as `run_python` and `run_node`.
3. Use `tokio::process` directly rather than constructing shell strings.
4. Enforce timeout, max output, working directory, environment, and safe-mode gates.
5. Store source snippets and outputs as session artifacts.
6. Prefer file-backed temporary execution over inline shell heredocs.

## Verification

```bash
cargo test interpreter
cargo test sandbox
cargo test tool_calling
cargo build
```

## Done Criteria

- Interpreter execution does not require shell string synthesis.
- Missing interpreters are hidden or produce clear unavailable results.
- Output is bounded, sanitized, and transcript-visible.
- Dangerous execution paths require permission.

