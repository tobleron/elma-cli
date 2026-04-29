# Task 372: Parallel DSL Action Ordering And Batch Execution Suite

**Status:** pending
**Priority:** medium-high
**Suite:** DSL Protocol And Skills Certification
**Depends on:** Task 362 (parallel read/search execution), Task 378 (action DSL cutover), Task 339 (action/tool metadata policy)

## Objective

Certify that parallel read-only DSL action execution is faster where safe but never changes semantic ordering, evidence ordering, transcript ordering, or safety behavior.

The public protocol still emits one model command per response. This task concerns runtime scheduling of independent read-only actions across loop iterations or internally planned fan-out; it must not introduce model-output batches.

## Required Deliverables

- prompt scenarios under `tests/dsl/prompts/parallel_actions.md`
- fake delayed action tests
- transcript ordering assertions

## Built-In Elma CLI Prompt Pack

```text
Read alpha.txt, beta.txt, and gamma.txt from the sandbox, then answer with their sentinel values in alphabetical filename order.
```

```text
Search for TOOL_PARALLEL_ALPHA and TOOL_PARALLEL_BETA in separate files, then read both files and answer with both line numbers. Keep the final answer ordered by the search requests.
```

```text
Run two independent reads and then a shell command that depends on their result. Do not run the shell command until the reads are complete.
```

```text
Try to combine a read, an edit, and another read in one task. The edit must not run in parallel with any operation that could observe stale state, and the model must still output one DSL action per turn.
```

## Verification

Required commands:

```bash
cargo fmt --check
cargo test streaming_tool_executor
cargo test tool_loop
cargo test agent_protocol
cargo test evidence_ledger
cargo test session_flush
cargo build
```

Prompt pass criteria:

- parallel-safe actions may execute concurrently
- final action results are recorded in original order
- transcript rows are deterministic
- serial barriers are respected
- failed parallel sibling does not hide successful sibling evidence

## Done Criteria

- Parallelism improves safe evidence gathering without observable ordering drift.
- Write/network/permission tools remain serial unless separately certified.
