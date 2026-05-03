# Task 480: Tool Equivalent Certification Scenarios From Knowledge Base

**Status:** Pending
**Priority:** MEDIUM
**Estimated effort:** 3-4 days
**Dependencies:** Task 386, Task 388, Task 479
**References:** source-agent parity and existing certification harness tasks

## Objective

Create local prompt scenarios that certify Elma's equivalents for source-agent tools using only offline fixtures.

## Implementation Plan

1. For each tool family in Task 386, write at least one scenario fixture.
2. Prefer deterministic local workspaces and local HTTP servers for optional network tools.
3. Assert:
   - correct tool discovery
   - rust-native preference
   - executor availability
   - bounded failure handling
   - transcript-visible decisions
   - grounded final answer
4. Integrate scenarios with the existing self-test/certification harness.

## Verification

```bash
cargo test certification
cargo test tool_calling
cargo build
```

## Done Criteria

- Every high-priority source-agent tool family has an offline scenario.
- Certification fails if a declared equivalent has no executor.
- Scenarios prove native tools are preferred over shell where applicable.

