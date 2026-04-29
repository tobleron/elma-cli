# Task 378: Action DSL Protocol And Executor Cutover

**Status:** pending
**Priority:** critical
**Suite:** Compact DSL Model-Output Migration
**Depends on:** Tasks 376, 377, 379, 382
**Blocks:** Tasks 381, 384, 365-375 reframe

## Objective

Replace the live model-to-executor action protocol in `run_tool_loop()` with a compact DSL command stream. The model must output exactly one action command per turn, and Elma must execute only typed, parsed, validated `AgentAction` values.

## Target DSL

```text
R path="relative/path"
L path="relative/path" depth=2
S q="search text" path="relative/path"
Y q="symbol_name" path="relative/path"
E path="relative/path"
---OLD
exact old text
---NEW
new text
---END
X
allowed verification command
---END
ASK
question
---END
DONE
summary
---END
```

## Required Rust Types

```rust
pub(crate) enum AgentAction {
    ReadFile { path: String },
    ListFiles { path: String, depth: u8 },
    SearchText { q: String, path: String },
    SearchSymbol { q: String, path: String },
    EditFile { path: String, old: String, new: String },
    RunCommand { command: String },
    Ask { question: String },
    Done { summary: String },
}
```

## Protocol Rules

- One response equals one command.
- No Markdown, JSON, XML, YAML, TOML, or prose.
- Paths are project-root-relative only.
- `L depth` defaults to `2` and caps at `5`.
- `X`, `ASK`, and `DONE` require block body plus `---END`.
- `E` requires `---OLD`, `---NEW`, and `---END`.
- Invalid DSL returns compact repair feedback and does not execute.
- `DONE` is the only successful finalization command.
- `ASK` returns control to the user with the question.

## Executor Mapping

- `R`: use `document_adapter::read_file_smart` after path validation.
- `L`: use a native workspace listing/tree helper; do not call shell `ls`.
- `S`: use direct `std::process::Command` invocation of `rg` or native search; no shell interpolation.
- `Y`: use direct symbol-oriented search with escaped query boundaries and bounded output.
- `E`: call the shared exact edit engine from Task 379.
- `X`: call the strict command policy executor from Task 379.
- `ASK`: produce a final user-facing question and stop the loop.
- `DONE`: produce the final answer and stop the loop.

## Implementation Steps

1. Add `AgentAction` AST, parser, renderer, and validation entrypoint.
2. Add an `execute_agent_action()` dispatcher that returns compact observations.
3. Modify `tool_loop.rs` to request assistant text DSL instead of provider-native `tools`.
4. Buffer streamed assistant text until parse/execution completes so raw DSL does not leak as prose.
5. Replace `ToolLoopModelTurn { tool_calls }` handling with `AgentAction` handling.
6. Keep provider structs that are still needed for HTTP compatibility, but remove them from live action planning.
7. Remove or quarantine live native tool-call fallback once tests pass.
8. Update prompt/docs references from tool-calling to DSL action protocol.

## Verification

Required commands:

```bash
cargo fmt --check
cargo test agent_protocol
cargo test tool_loop
cargo test tool_calling
cargo test stop_policy
cargo check --all-targets
```

Required coverage:

- parse and execute each valid action
- invalid DSL returns repair observation
- JSON/native tool-call-looking output is rejected
- `ASK` stops with question
- `DONE` stops with summary
- raw action commands do not render as final assistant prose
- observations are appended to model context and transcript

## Done Criteria

- The live action path no longer depends on model-produced JSON or provider-native tool-call arguments.
- Every action goes through parse, semantic validation, safety validation, execution, and observation.
- Rollback is possible through the checkpoint commit, not a permanent runtime JSON fallback.

## Anti-Patterns

- Do not execute raw model text.
- Do not keep a hidden permissive native tool-call path.
- Do not expand the DSL with batching, variables, conditionals, loops, or nested actions.
