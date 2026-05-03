# Task 473: Provider Fault Injection And Error Recovery Harness

**Status:** pending
**Source patterns:** Goose provider-error-proxy, Open Interpreter context-window handling, Qwen-code integration fixtures
**Depends on:** completed Task 303 (offline-first architecture), Task 381 (transcript operational visibility)

## Summary

Add a local provider fault-injection harness that simulates context-length errors, malformed streams, network timeouts, rate limits, auth errors, server errors, and truncated tool-call responses.

## Why

Elma must be reliable on local and OpenAI-compatible providers. Provider failures are currently hard to reproduce. A deterministic fault harness lets recovery behavior be tested without depending on live services.

## Implementation Plan

1. Add a local proxy/test server that implements the provider endpoints Elma uses.
2. Add scenario fixtures for stream interruption, invalid JSON, context overflow, timeout, and provider-specific error bodies.
3. Verify compaction, retry, clean-context finalization, and visible transcript notices.
4. Keep the harness out of normal runtime dependencies when possible.

## Success Criteria

- [ ] Tests can reproduce context-length and malformed-stream failures deterministically.
- [ ] Elma surfaces provider stop reasons in transcript rows.
- [ ] Recovery avoids infinite retries and preserves evidence already collected.
- [ ] The harness works offline.
- [ ] Scenario tests document expected recovery behavior.

## Anti-Patterns To Avoid

- Do not require a paid provider for failure tests.
- Do not hide recovery decisions in trace-only logs.
- Do not respond to provider faults by bloating prompts.
