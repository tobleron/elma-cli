# Task 368: Filesystem Mutation DSL E2E Self-Test Suite

**Status:** pending
**Priority:** high
**Suite:** DSL Protocol And Skills Certification
**Depends on:** Task 326 (exact edit engine), Task 365 (DSL protocol self-test harness), Task 378 (action DSL cutover), Task 379 (DSL safety)

## Objective

Certify filesystem mutation through the model-facing DSL `E` exact replace action and any remaining internal mutation adapters end to end: exact edit, stale-read behavior, ambiguous replacement rejection, rollback/snapshot behavior where supported, and verification after mutation.

Do not reintroduce model-output JSON write/patch commands in this task. New file creation or multi-file patching needs an explicit future DSL extension decision; it must not be smuggled through unrestricted `X`.

## Required Deliverables

- mutation fixture workspace under `_stress_testing/dsl_protocol_lab/mutation/`
- prompt scenarios under `tests/dsl/prompts/mutation_actions.md`
- rollback/stale-read regression tests
- documentation of the `E` safety contract and any internal mutation adapter contract

## Built-In Elma CLI Prompt Pack

```text
Read edit_fixture.txt, replace the single occurrence of DSL_MUTATION_OLD with DSL_MUTATION_NEW, then read the file again to verify the edit. Do not use `X` for the edit.
```

```text
Read ambiguous_edit_fixture.txt and try to replace the word DUPLICATE with CHANGED. If the edit is unsafe because there are multiple matches, do not modify the file; explain the error and ask for more context.
```

```text
Try to edit edit_fixture.txt using an OLD block that does not exist. Confirm that no files changed after the failed edit.
```

```text
After reading stale_fixture.txt, assume it may have changed externally. Re-read before editing, then make a safe single replacement and verify it.
```

## Self-Improvement Loop Protocol

1. Run each prompt against disposable fixtures.
2. Diff the sandbox before and after.
3. If an unsafe mutation succeeds, stop and fix the safety gate.
4. If a safe mutation fails, inspect parser/validator/executor mismatch.
5. Add automated regression tests for the exact failure.
6. Re-run until safe prompts pass and unsafe prompts fail safely.

## Verification

Required commands:

```bash
cargo fmt --check
cargo test edit_engine
cargo test agent_protocol
cargo test tool_loop
cargo test execution_steps_edit
cargo build
```

Prompt pass criteria:

- all writes stay inside sandbox
- every mutation is verified by a follow-up read/search
- stale or ambiguous edits fail without modifying files
- failed edits leave the file unchanged
- final answer states exactly which files changed
- session transcript contains mutation evidence

## Done Criteria

- DSL `E` has at least one successful and one safe-failure prompt.
- No mutation tool can bypass read-before-write policy where required.
- The matrix marks mutation actions/adapters certified only after prompt and automated tests pass.
