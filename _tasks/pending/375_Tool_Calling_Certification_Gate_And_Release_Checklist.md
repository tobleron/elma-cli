# Task 375: DSL Protocol Certification Gate And Release Checklist

**Status:** pending
**Priority:** critical
**Suite:** DSL Protocol And Skills Certification
**Depends on:** Tasks 364, 365, 366, 367, 368, 369, 370, 371, 372, 373, 374

## Objective

Create the final certification gate that proves every DSL action and skill/formula path is declared, executable or intentionally hidden, policy-gated, tested, transcript-visible, evidence-coherent, and documented. Remaining internal tools/adapters must also be classified so they cannot be mistaken for model-callable JSON tools.

## Required Deliverables

- `_scripts/certify_dsl_protocol.sh`
- `docs/dsl/CERTIFICATION_REPORT.md`
- updated `docs/dsl/DSL_PROTOCOL_MATRIX.md`
- pass/fail checklist for every action, compatibility tool, and skill/formula

## Certification Dimensions

Every model-callable DSL action must be marked:

- certified
- certified with limitations
- disabled intentionally
- failed with linked task

Every remaining compatibility tool/adapter must be marked:

- internal-only
- disabled intentionally
- declaration-only pending implementation
- obsolete after DSL migration
- failed with linked task

Certification requires:

- DSL parser/validator contract tests
- executor tests
- prompt scenario pass
- permission policy pass
- transcript/session/evidence pass
- concurrency classification
- failure-mode coverage
- no prompt-core modification unless explicitly approved

## Built-In Elma CLI Prompt Pack

Final manual certification prompts:

```text
Run a full DSL protocol self-audit of this project. Identify every callable DSL action, prove each has a parser, validator, and executor, and list any internal tool/adapter that is disabled, declaration-only, or obsolete. Use source evidence.
```

```text
In the sandbox, complete a task that requires search, read, a safe verification command, and a final grounded DONE. Keep all work inside the sandbox.
```

```text
In the sandbox, complete a task that safely applies an exact edit, verifies it, and then summarizes the file change. Include exact file paths and verification evidence.
```

```text
Explain whether Elma's current DSL action protocol is production-ready. Base your answer only on the certification matrix and self-test reports.
```

## Verification

Required commands:

```bash
bash -n _scripts/certify_dsl_protocol.sh
cargo fmt --check
cargo test -p elma-tools
cargo test agent_protocol
cargo test tool_registry
cargo test tool_loop
cargo test streaming_tool_executor
cargo test stop_policy
cargo test evidence_ledger
cargo test session_flush
cargo test skills
cargo build
```

Certification script requirements:

- fails if any executable action lacks a parser, validator, or executor
- fails if any certified action/tool lacks prompt evidence
- fails if any certified action/tool lacks automated tests
- fails if prompt-core changed during certification without explicit approval
- prints a concise summary table
- links the detailed report path

## Done Criteria

- The full suite gives a single trustworthy answer: which DSL actions and compatibility tools are production-ready and which are not.
- All DSL/action failures discovered by prompt tests are either fixed or linked to pending tasks.
- The project has a repeatable gate for future protocol additions.
