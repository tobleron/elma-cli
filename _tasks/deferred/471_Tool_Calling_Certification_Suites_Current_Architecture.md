# Task 471: Tool Calling Certification Suites For Current Architecture

**Status:** pending
**Priority:** HIGH
**Estimated effort:** 4-7 days
**Depends on:** completed Task 381, completed Task 393, completed Task 430, Task 446, Task 470
**References:** `tests/fixtures/ui_parity/`, `src/tool_loop.rs`, `src/tool_calling.rs`, `src/session_flush.rs`, `src/evidence_ledger.rs`, `src/claude_ui/`

## Problem

The DSL branch created useful certification ideas: smoke prompts, session regression scans, protocol matrices, and evidence/transcript checks. The DSL-specific parts should stay retired, but the current tool-calling architecture still needs the same kind of repeatable certification.

Right now, many regressions are only caught by scattered unit tests or manual sessions:

- tool calls not executed
- final answers unsupported by evidence
- permission gate hangs
- unsafe parallel scheduling
- transcript/session drift
- oversized tool output corrupting context
- tool visibility only present in traces

## Objective

Build certification suites for the current JSON/tool-calling architecture. The suites should exercise real current behavior without depending on DSL grammar, DSL action lines, or DSL repair.

## Non-Goals

- Do not add DSL protocol tests.
- Do not require network by default.
- Do not mutate real project files outside a disposable fixture workspace.
- Do not make certification depend on one specific local model.
- Do not bury failures in trace-only logs.

## Suite Structure

Create a small certification tree:

```text
tests/certification/
  fixtures/
  prompts/
  expected/
  reports/          # gitignored if generated
```

Add scripts only when they are thin wrappers around cargo tests or a documented manual harness:

```text
_scripts/run_tool_calling_certification.sh
_scripts/session_regression_scan.sh
```

The script names may differ, but they must not contain DSL-specific terminology.

## Required Suites

### 1. Tool Execution And Evidence Grounding

Prove that the model/tool loop cannot satisfy evidence-requiring requests with unsupported final answers.

Coverage:

- read exact file content
- search then read
- observe metadata before read
- command execution with short output
- command execution with large persisted output
- failed command followed by corrected attempt or honest failure

Pass criteria:

- final answer cites or uses observed evidence
- evidence ledger has matching entries
- transcript shows visible tool rows
- raw large output is artifact-referenced, not injected wholesale

### 2. Permission And Safe Mode

Prove that permission behavior is non-hanging, explainable, and policy-aligned.

Coverage:

- safe read-only commands
- destructive shell command blocked or prompted
- workspace write through edit/write
- denied permission produces a grounded final answer
- non-interactive mode does not hang waiting for TUI input

Pass criteria:

- permission request rows are visible
- denial is reflected in final answer
- no hidden execution after denial
- safe mode policy is deterministic under test

### 3. Transcript And Session Coherence

Prove that user-visible runtime history and durable session state agree.

Coverage:

- user prompt persisted once
- assistant final answer persisted once
- tool rows visible in transcript
- operational rows from Task 381 visible
- compact boundary visible
- session index points to the canonical transcript
- after Task 430, no duplicate transcript files are created

Pass criteria:

- session reload/index sees the same transcript path
- no duplicate final-answer/user-prompt artifacts in normal mode
- legacy session loading remains covered separately

### 4. Parallel Tool Ordering

Prove that safe parallelism does not change semantic ordering or safety behavior.

Coverage:

- independent read/search/observe operations
- dependent read then edit
- failed sibling tool plus successful sibling tool
- permission-gated tool blocks only the unsafe branch

Pass criteria:

- final answer preserves requested order
- transcript rows are deterministic
- write tools are serialized
- failed tools do not hide successful evidence

### 5. Context And Output Budget

Prove that long outputs and long sessions remain bounded.

Coverage:

- large shell output
- large file read
- repeated tool results
- compaction threshold
- final answer extraction after compaction

Pass criteria:

- context remains under budget
- raw output is persisted once
- final answer remains grounded
- compaction events are visible

## Implementation Plan

### Phase 1: Fixture Workspace

Create a disposable fixture workspace containing:

- small text files with sentinel strings
- nested directories
- a large generated text file
- symlink fixture where supported
- files safe to edit
- files that should remain protected

Do not use the real repo as the mutation target.

### Phase 2: Unit And Integration Tests

Prefer Rust tests for deterministic behavior:

- tool executor tests
- permission policy tests
- session persistence tests
- evidence ledger tests
- streaming/parallel scheduler tests

Use prompt/manual harness only where live-model behavior is genuinely required.

### Phase 3: Session Regression Scanner

Add a read-only scanner for existing sessions that reports:

- missing transcript
- missing final answer
- final answer with no evidence for evidence-required turn
- hidden stop reason
- duplicate transcript/final answer artifacts
- missing artifact target
- permission prompt without resolution

Output should be concise TOML or markdown. It must not modify sessions.

### Phase 4: Certification Command

Add a single local command/script that runs the deterministic suites:

```bash
bash _scripts/run_tool_calling_certification.sh --offline
```

The command should:

- build fixtures
- run relevant `cargo test` filters
- optionally run manual/live prompts when explicitly requested
- write a compact report

## Files To Audit

| File | Reason |
|------|--------|
| `src/tool_loop.rs` | Tool-loop stop policy, evidence gate, finalization |
| `src/tool_calling.rs` | Tool executor behavior |
| `src/streaming_tool_executor.rs` | Parallel scheduling |
| `src/permission_gate.rs` | Permission prompts and denial |
| `src/safe_mode.rs` | Permission mode behavior |
| `src/session_flush.rs` | Current transcript persistence |
| `src/session_index.rs` | Session reload/index checks |
| `src/evidence_ledger.rs` | Evidence grounding |
| `src/tool_result_storage.rs` | Large output persistence |
| `tests/fixtures/ui_parity/` | Existing UI parity fixture pattern |

## Success Criteria

- [ ] Offline certification runs without network or a live model.
- [ ] Live-model smoke mode is optional and clearly marked.
- [ ] Certification failure classes are stable and actionable.
- [ ] Tool execution, evidence grounding, permission, session, parallel ordering, and context budget are all covered.
- [ ] Generated reports include session/transcript/artifact paths where relevant.
- [ ] The suite does not depend on DSL files, DSL grammar, or DSL terminology.

## Verification

```bash
bash -n _scripts/run_tool_calling_certification.sh
cargo test tool_calling
cargo test permission_gate
cargo test safe_mode
cargo test session
cargo test evidence_ledger
cargo test streaming_tool_executor
cargo build
```

Manual/live mode, only when explicitly requested:

```bash
ELMA_SELF_TEST_BASE_URL=http://127.0.0.1:8080 \
ELMA_SELF_TEST_MODEL=local-model \
bash _scripts/run_tool_calling_certification.sh --live
```

## Anti-Patterns To Avoid

- Do not test by matching polished answer wording.
- Do not require a specific local model for offline certification.
- Do not mutate real repo files during certification.
- Do not use trace logs as the only source of truth.
- Do not recover DSL protocol tests under a new name.
