# 563 — Add Integration Tests for Full Tool-Calling Pipeline

- **Priority**: High
- **Category**: Testing
- **Depends on**: 552 (split tool_calling.rs), 553 (tool arg validation)
- **Blocks**: None

## Problem Statement

The codebase has unit tests for individual components (stop policy, evidence ledger, shell preflight, tool registry, JSON parsing) but no integration tests that exercise the full tool-calling pipeline end-to-end:

- `run_tool_loop()` → model request → SSE parsing → tool execution → evidence recording
- `run_tool_calling_pipeline()` → system prompt assembly → tool loop → final answer
- Classification → execution ladder → work graph → tool loop → finalization

The existing `tests/dsl/` directory has DSL protocol tests but doesn't test the HTTP round-trip or tool execution with real filesystem operations.

## Why This Matters for Small Local LLMs

Integration tests are the only way to catch issues like:
- A system prompt change that causes the model to call the wrong tool
- An argument validation change that breaks existing valid tool calls
- A stop policy change that prevents the model from completing reasonable tasks
- An evidence ledger change that causes the model to lose track of collected data

Small models are more sensitive to prompt and tool schema changes than large models.

## Current Behavior

Testing is primarily unit-level. Scenario tests exist in `scenarios/` but require a running LLM backend. There are no integration tests that:
1. Mock the HTTP layer and replay recorded model responses
2. Exercise tool execution with real filesystem operations
3. Verify the full lifecycle from system prompt → tool loop → final answer

## Recommended Target Behavior

Create an integration test harness that:

1. **Mock LLM responses**: Use recorded JSON fixtures instead of live HTTP calls
2. **Temp workspace**: Create temporary directories for filesystem operations
3. **Full pipeline**: Test classification → assessment → tool loop → finalization
4. **Tool execution verification**: Check that tool results are correct for given inputs
5. **Error path testing**: Verify behavior when tools fail, budget exceeded, stagnation

### Test Scenarios

| Scenario | Description | Expected Outcome |
|----------|-------------|-----------------|
| Read file | User asks to read a file | Model calls read, answer includes file content |
| Search code | User asks to find a pattern | Model calls search, answer includes results |
| Simple chat | User asks a conversational question | Model responds without tools |
| Stagnation stop | Model repeats same tool call 8 times | Stop policy triggers, evidence-based final answer |
| Budget stop | Model exceeds iteration budget | Stop policy triggers, timeout message |
| Shell with preflight | Model tries rm -rf | Preflight blocks, model receives error |
| Shell with permission | Model tries mv (caution) | Permission gate prompts, model confirms |
| Evidence grounding | Model makes unsupported claim | Evidence enforcement catches ungrounded claim |
| Auto compaction | Context exceeds limit | Compaction triggers, model continues |
| Tool failure recovery | Tool reports error | Model retries with different approach |

## Source Files That Need Modification

- `src/tool_loop.rs` — May need minor changes to support mock HTTP client
- `src/tool_calling.rs` — May need factory/test helpers
- `src/llm_provider.rs` — May need test HTTP client injection

## New Files/Modules

- `tests/integration/mod.rs` — Integration test harness
- `tests/integration/mock_llm.rs` — Mock LLM backend with recorded responses
- `tests/integration/fixtures/` — Recorded model responses (JSON)
- `tests/integration/pipeline_tests.rs` — Full pipeline integration tests
- `tests/integration/tool_execution_tests.rs` — Tool execution verification

## Step-by-Step Implementation Plan

1. Create `MockLlmBackend` that replays recorded responses:
   ```rust
   struct MockLlmBackend {
       responses: Vec<RecordedResponse>,
       current: usize,
   }
   
   impl MockLlmBackend {
       fn from_fixture(path: &str) -> Self { ... }
       fn next_response(&mut self) -> ChatCompletionResponse { ... }
   }
   ```

2. Record real model responses for test scenarios:
   - Run each test scenario against a real LLM
   - Save the complete request/response sequence
   - Anonymize any sensitive data
   - Store as JSON fixtures in `tests/integration/fixtures/`

3. Create `TempWorkspace` helper:
   ```rust
   struct TempWorkspace {
       root: TempDir,
   }
   impl TempWorkspace {
       fn new() -> Self { ... }
       fn create_file(&self, path: &str, content: &str) { ... }
       fn create_dir(&self, path: &str) { ... }
   }
   ```

4. Write pipeline integration tests for each scenario
5. Add tool execution verification (check actual filesystem state after tool execution)
6. Run integration tests in CI (requires no external services)
7. Add regression test: any new feature must pass integration test suite

## Recommended Crates

- `tempfile` — already a dependency; for temporary workspaces
- `serde_json` — for fixture serialization
- `tokio::test` — for async integration tests

## Validation/Sanitization Strategy

- Integration tests never make real HTTP calls
- All filesystem operations use temporary directories
- Tests clean up after themselves (TempDir drop)
- Recorded responses are reviewed for sensitive data before committing

## Testing Plan

The integration tests ARE the testing plan. Additionally:
- Integration tests run in CI
- Tests complete in <30 seconds (total)
- Fixtures are versioned alongside code
- New features require integration test updates

## Acceptance Criteria

- 10 integration test scenarios passing
- All tests use mock LLM backend (no real HTTP)
- Tool execution verification checks actual filesystem state
- Integration tests run in CI
- Test fixtures are versioned in the repository
- Smoke test: `cargo test --test integration` passes

## Risks and Migration Notes

- **Fixture maintenance**: Recorded responses may become outdated when tool schemas change. Add version metadata to fixtures and validate on load.
- **Mock fidelity**: The mock backend may not reproduce all edge cases (streaming behavior, partial responses, timeouts). Supplement with fuzzing tests (Task 561).
- **Determinism**: Integration tests must be deterministic. Use fixed random seeds and recorded responses.
- Start with 3 critical scenarios (read, search, shell) and expand to 10.
