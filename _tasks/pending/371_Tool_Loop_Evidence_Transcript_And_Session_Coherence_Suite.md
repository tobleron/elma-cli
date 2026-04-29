# Task 371: DSL Loop Evidence Transcript And Session Coherence Suite

**Status:** pending
**Priority:** high
**Suite:** DSL Protocol And Skills Certification
**Depends on:** Task 338 (event log) preferred, Task 365 (DSL protocol self-test harness), Task 378 (action DSL cutover)

## Objective

Certify that parsed DSL actions integrate coherently with the action loop, evidence ledger, transcript rows, session flush, stop policy, and final answer grounding.

## Required Deliverables

- prompt scenarios under `tests/dsl/prompts/dsl_loop_coherence.md`
- automated tests for evidence/session persistence
- transcript audit checklist

## Built-In Elma CLI Prompt Pack

```text
Answer this only after collecting evidence: which sandbox file contains DSL_EVIDENCE_ALPHA, and what is the exact line? Your final answer must mention the evidence you used.
```

```text
Try to finish without evidence, then correct by using search/read before finalizing. The system should prevent unsupported DONE.
```

```text
Run a search that returns no matches, then use a different evidence strategy to find DSL_EVIDENCE_BETA. Explain the strategy change.
```

```text
Perform a read, a search, and a safe shell command. Then summarize the evidence ledger in your own words and answer the original question.
```

## Verification

Required commands:

```bash
cargo fmt --check
cargo test stop_policy
cargo test evidence_ledger
cargo test session_flush
cargo test tool_result_storage
cargo test agent_protocol
cargo test tool_loop
cargo build
```

Prompt pass criteria:

- every evidence-producing action creates a visible transcript row
- evidence ledger entries correspond to action results
- session artifacts contain action result flushes
- final answer does not cite ASK/DONE or repair feedback as evidence
- unsupported-DONE loop guard works
- no compaction step drops required evidence

## Done Criteria

- Action output, evidence, transcript, and final answer agree.
- Failures produce a precise regression test.
- Session resume can still explain what happened.
