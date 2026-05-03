# Task 366: DSL Action Declaration Executor Contract Tests

**Status:** pending
**Priority:** critical
**Suite:** DSL Protocol And Skills Certification
**Depends on:** Task 364 (DSL protocol coverage matrix), Task 339 (action/tool metadata policy), Task 377 (DSL parser/error model)

## Objective

Guarantee that every DSL action exposed to a model has a parser, validator, executor, and recoverable observation. Any remaining compatibility tool schema must either be internal-only/disabled or have matching executor coverage.

## Required Deliverables

- unit tests for DSL action parsing and validation
- main-crate contract tests for action executor dispatch
- compatibility tests for any remaining `elma-tools` schemas
- an action/executor parity report linked from the DSL protocol matrix

## Contract Requirements

For every executable DSL action:

- declared required fields must match parser and executor requirements
- optional fields such as `L depth` must have documented defaults
- invalid DSL must fail with a structured repair error
- missing required fields must fail before side effects
- unknown fields must be rejected consistently
- executor output must render compact observations coherently

For every non-executable or compatibility-only tool:

- metadata must hide it or mark it declaration-only
- discovery must not say it is callable by the model

## Built-In Elma CLI Prompt Pack

```text
Inspect Elma's DSL action declarations and executor dispatch. Find any action whose parser/validator exists but whose executor is missing or incomplete. Use exact source evidence and recommend the smallest fix.
```

```text
Inspect editing and web-related capabilities, then explain which are model-callable DSL actions, internal-only adapters, disabled, or missing executors. Do not assume a declared schema means execution works.
```

```text
Deliberately make no filesystem changes. Inspect the code and explain how Elma handles invalid DSL for read, search, verification command, ask, and done actions.
```

## Self-Improvement Loop Protocol

1. Run the prompt.
2. If Elma claims an action/tool works without executor evidence, add or fix a contract test.
3. If the model discovers a declaration-only tool as callable, fix action/tool metadata or discovery output.
4. Re-run until the answer distinguishes declaration, availability, and executability.

## Verification

Required commands:

```bash
cargo fmt --check
cargo test -p elma-tools
cargo test agent_protocol
cargo test tool_registry
cargo test tool_loop
cargo test streaming_tool_executor
cargo build
```

Required coverage:

- executable action without executor fails a test
- parser/validator-only action without executor fails a test
- declaration-only tool is not exposed as callable
- each action has a minimal valid DSL fixture
- each action has a missing-required-field fixture
- invalid DSL returns structured repair failure
- discovery output is truthful about availability

## Done Criteria

- No callable DSL action can return `Unknown action` in normal operation.
- Parser/validator/executor drift is test-detected.
- Compatibility schema drift between `elma-tools` and remaining executors is test-detected.
- The model-facing discovery path is truthful.
