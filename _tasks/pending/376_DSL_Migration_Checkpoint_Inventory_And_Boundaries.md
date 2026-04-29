# Task 376: DSL Migration Checkpoint Inventory And Boundaries

**Status:** pending
**Priority:** critical
**Suite:** Compact DSL Model-Output Migration
**Depends on:** none
**Blocks:** Tasks 377-384

## Objective

Create the migration baseline for replacing all model-produced structured JSON with compact Rust-native DSL outputs. This task does not implement protocol changes; it proves the current state, records rollback boundaries, and classifies every JSON use by whether it is model output or legitimate internal/external plumbing.

## Required Deliverables

- A checkpoint git commit before any DSL migration code changes.
- `docs/dsl/MODEL_OUTPUT_JSON_INVENTORY.md`
- `docs/dsl/JSON_BOUNDARIES.md`
- An inventory fixture such as `tests/dsl/model_output_json_inventory.toml`
- A migration note linking every affected prompt/profile/module to a follow-up task.

## Boundary Rules

Classify each JSON use into exactly one category:

- `model_output`: the LLM is instructed to emit structured JSON; must migrate to DSL.
- `provider_wire`: OpenAI-compatible/Anthropic/local-server HTTP payloads; may stay JSON.
- `local_state`: sessions, traces, reports, caches, runtime records; migrate only when a task explicitly requires it.
- `config`: user/runtime configuration; should be TOML-first.
- `third_party_contract`: dependency/API format outside Elma's control; may stay JSON.
- `test_fixture`: fixture exists only to test compatibility or legacy behavior; keep only if it still serves a current boundary.

Do not count JSON literals in historical completed task docs as active model-output debt.

## Inventory Scope

Inspect at least:

- `src/defaults_core.rs`, `src/defaults_evidence.rs`, `src/defaults_evidence_core.rs`, `src/defaults_router.rs`
- `src/prompt_constants.rs`, `src/prompt_core.rs`
- `src/intel_units/**`, `src/intel_trait.rs`
- `src/ui/ui_chat.rs`, `src/routing_parse.rs`, `src/json_parser*.rs`, `src/json_error_handler/**`
- `src/tool_loop.rs`, `src/tool_calling.rs`, `src/orchestration_core.rs`, `src/orchestration_helpers/**`
- `config/**`, especially profile prompts and grammar mappings
- `docs/**` and `_tasks/pending/**` for active task references that would preserve JSON model output

## Implementation Steps

1. Confirm the checkpoint commit exists and record its hash in the inventory document.
2. Run source searches for `Return ONLY`, `valid JSON`, `serde_json`, `chat_json_with_repair`, `parse_json_loose`, `json_program_grammar`, `tool_calls`, and `function.arguments`.
3. Build the category inventory with file path, symbol/profile name, model role, current schema shape, owner task, and migration priority.
4. Mark legitimate non-model JSON boundaries so future cleanup does not waste effort on provider/session plumbing.
5. Identify all live model-output JSON prompts and assign them to Tasks 378, 380, 381, 382, or 384.
6. Update any active/pending task references that incorrectly describe JSON as the desired final model-output format.

## Verification

Required commands:

```bash
rg -n "Return ONLY.*JSON|valid JSON|Program JSON|chat_json_with_repair|parse_json_loose|json_program_grammar|tool_calls|function.arguments|serde_json" src config docs _tasks/pending
cargo fmt --check
```

Required checks:

- Every active model-output JSON prompt has a target DSL migration task.
- Provider-wire and local-state JSON are documented as intentional boundaries.
- The checkpoint commit hash is recorded.
- No source behavior changes are included in this task.

## Done Criteria

- The migration has an exact rollback point.
- The team can tell which JSON uses must disappear and which are acceptable.
- Later DSL tasks can be implemented without rediscovering the same surfaces.

## Anti-Patterns

- Do not start editing prompts or parsers in this task.
- Do not declare all JSON bad; only model-produced structured JSON is the primary target.
- Do not leave ambiguous categories such as "maybe".
