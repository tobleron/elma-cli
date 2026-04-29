# Task 364: DSL Protocol Coverage Matrix And Baseline Audit

**Status:** pending
**Priority:** critical
**Suite:** DSL Protocol And Skills Certification
**Depends on:** Task 376 (DSL inventory), Task 339 (action/tool metadata policy) is preferred but not required to start

## Objective

Create the canonical inventory of every model-callable DSL command, remaining internal tool/adapter, and skill/formula path, then classify what is declared, executable, tested, permission-gated, transcript-visible, evidence-producing, and integrated with the rest of the architecture.

This is the entry task for the DSL protocol certification suite. All later tasks must reference the matrix created here.

## Required Deliverables

- `docs/dsl/DSL_PROTOCOL_MATRIX.md`
- `docs/dsl/SELF_TEST_PROMPTS.md`
- a machine-readable fixture such as `tests/dsl/protocol_matrix.toml`
- a short backlog reconciliation note listing duplicated, stale, or already-covered action/tool tasks

## Matrix Requirements

For every model-callable DSL action, record:

- command name
- AST variant
- parser location
- validator location
- executor location
- required and optional fields
- read/write/network/external-process/conversation-state risk
- permission behavior
- concurrency behavior
- evidence ledger behavior
- transcript visibility behavior
- session persistence behavior
- direct automated tests
- direct `elma-cli` prompt tests
- known gaps

For every tool declared under `elma-tools/src/tools/`, record:

- tool name
- declaration file
- executor location
- compatibility state: live DSL-backed, internal-only, disabled, declaration-only, or obsolete
- schema fields and required fields
- read/write/network/external-process risk
- permission behavior
- concurrency behavior
- evidence ledger behavior
- transcript visibility behavior
- session persistence behavior
- direct automated tests
- direct `elma-cli` prompt tests
- known gaps

For skills and formulas, record:

- skill/formula name
- selection path
- expected tool families
- evidence requirements
- direct prompt tests
- known failure modes

## Built-In Elma CLI Prompt Pack

Run these in a clean sandbox workspace and capture the session ids.

```text
Inspect this project and produce a table of every model-callable DSL command and remaining internal tool/adapter. For each entry, say whether it reads files, writes files, runs verification commands, uses network, or only updates conversation state. Use local evidence only.
```

```text
Find where Elma parses DSL actions and where it dispatches them. Compare them and identify any action or compatibility tool that cannot actually execute. Use exact file paths and line evidence.
```

```text
Inspect the skill and formula system. List each built-in skill or formula and explain which DSL action families it is expected to use. Use file evidence only.
```

## Self-Improvement Loop Protocol

For each prompt:

1. Run the prompt in `elma-cli`.
2. Save the session id and transcript path.
3. Compare the answer against source evidence.
4. If Elma misses a declared action, hallucinates a command/tool, or fails to cite files, implement the smallest architectural or prompt-free code fix.
5. Add a regression test or matrix assertion.
6. Re-run until the answer is correct twice in a row on the same model.

Do not modify `src/prompt_core.rs` for this task.

## Verification

Required commands:

```bash
cargo fmt --check
cargo test -p elma-tools tool_registry
cargo test agent_protocol
cargo test tool_registry
cargo test skills
cargo build
```

Required checks:

- every `AgentAction` variant is represented in the matrix
- every executor arm in the DSL action dispatcher is represented in the matrix
- every remaining `elma-tools/src/tools/*.rs` module is represented as live, internal, disabled, declaration-only, or obsolete
- every built-in skill/formula in `src/skills.rs` is represented
- every gap has a linked pending task or a documented reason
- prompt transcripts are linked from `docs/dsl/SELF_TEST_PROMPTS.md`

## Done Criteria

- The matrix is complete and source-grounded.
- Later certification tasks can reference one stable inventory.
- No action/tool is marked production-ready without executor and verification evidence.
