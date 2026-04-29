# Task 365: Elma CLI DSL Protocol Self-Test Harness

**Status:** pending
**Priority:** critical
**Suite:** DSL Protocol And Skills Certification
**Depends on:** Task 364 (DSL protocol coverage matrix)

## Objective

Build a repeatable local harness for running `elma-cli` prompt scenarios against a controlled sandbox workspace. The harness must support a self-improvement loop: run prompt, capture transcript/session, classify DSL/action failure, implement fix, rerun, and archive evidence.

## Required Deliverables

- `_stress_testing/dsl_protocol_lab/` fixture workspace
- `tests/dsl/prompts/*.md` prompt packs
- `_scripts/run_dsl_protocol_self_tests.sh`
- `docs/dsl/SELF_TEST_HARNESS.md`
- generated run reports under `sessions/` or a documented artifacts directory

## Harness Requirements

The harness must:

- create a disposable workspace with text files, source files, nested directories, symlinks, protected fixtures, and expected outputs
- run only local/offline tests by default
- accept `ELMA_SELF_TEST_BASE_URL` and `ELMA_SELF_TEST_MODEL`
- record command, prompt, session id, transcript path, pass/fail result, and failure class
- support manual interactive use and scripted smoke use
- never require network access by default

If `elma-cli` does not currently have a robust non-interactive prompt mode, this task must add one or create a documented manual fallback plus a pending implementation task for true scripted mode.

## Built-In Elma CLI Prompt Pack

Store these under `tests/dsl/prompts/`.

```text
You are in a sandbox workspace. First inspect the files, then tell me the exact path and contents of the file that contains the phrase DSL_SENTINEL_ALPHA. Do not guess.
```

```text
Search the sandbox for DSL_SENTINEL_BETA, read the file containing it, and answer with the filename and line number.
```

```text
Create a short checklist of which tools you used and what each tool proved. Use only evidence from this turn.
```

```text
Try to answer this without tools only if it is already known from this session: what is the checksum marker in checksum_fixture.txt? If you do not know it, inspect the file.
```

## Failure Classes

The harness must classify failures into:

- schema mismatch
- invalid DSL
- action not executed
- wrong action selected
- executor failure
- permission failure
- stale or missing evidence
- transcript/session persistence failure
- final answer unsupported
- loop/stagnation
- UI/manual-only limitation

## Verification

Required commands:

```bash
bash -n _scripts/run_dsl_protocol_self_tests.sh
cargo fmt --check
cargo test agent_protocol
cargo test tool_loop
cargo test session_flush
cargo test evidence_ledger
cargo build
```

If a scriptable CLI mode exists:

```bash
ELMA_SELF_TEST_BASE_URL=http://127.0.0.1:8080 ELMA_SELF_TEST_MODEL=test-model bash _scripts/run_dsl_protocol_self_tests.sh --dry-run
```

Required checks:

- dry run does not contact a provider
- sandbox creation is idempotent
- reports include session/transcript locations
- failures are categorized with a stable label
- manual prompt instructions are clear enough to run directly in `elma-cli`

## Done Criteria

- A developer can run the prompt suite against a local model and get a structured report.
- Every later task can add prompts to this harness.
- The harness never mutates real project files except the sandbox and documented artifacts.
