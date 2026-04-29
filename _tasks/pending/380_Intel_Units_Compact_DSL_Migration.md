# Task 380: Intel Units Compact DSL Migration

**Status:** pending
**Priority:** critical
**Suite:** Compact DSL Model-Output Migration
**Depends on:** Tasks 376, 377, 382
**Blocks:** Task 384

## Objective

Replace model-produced JSON in Elma's intel units with compact task-specific DSL outputs. The goal is to reduce cognitive load on local models without weakening semantic validation, retries, or fallback behavior.

## Migration Principle

Each intel unit gets the smallest grammar that represents its job:

- one-token DSL for simple classifications
- compact key/value lines for bounded records
- repeated prefixed lines for lists
- block DSL only where multiline text is necessary

Do not force all intel outputs into the action DSL.

## Initial DSL Families

Examples to refine during implementation:

### Verdict DSL

```text
OK reason="short reason"
RETRY reason="short reason"
REVISE reason="short reason"
```

### Router DSL

```text
ROUTE name=SHELL confidence=0.73 entropy=0.21 evidence=yes
```

### Formula DSL

```text
FORMULA primary=inspect_reply alt=execute_reply reason="needs evidence"
```

### Scope DSL

```text
SCOPE objective="inspect tool path"
F path="src/tool_loop.rs"
F path="src/tool_calling.rs"
Q text="tool_calls"
END
```

### Selection DSL

```text
ITEM value="src/main.rs"
ITEM value="src/lib.rs"
REASON text="most likely entry points"
END
```

Exact syntax should be finalized in code and documented once parser tests lock it.

## Required Surfaces

Migrate live model-output JSON in:

- `src/intel_units/**`
- `src/intel_trait.rs`
- `src/defaults_core.rs`
- `src/defaults_evidence.rs`
- `src/defaults_evidence_core.rs`
- `src/defaults_router.rs`
- `src/prompt_constants.rs`
- orchestration reviewers/critics/repair helpers that currently call `chat_json_with_repair`

## Implementation Steps

1. Use Task 376 inventory to prioritize live units first.
2. Define AST types for each DSL family and conversion into existing Rust domain types.
3. Add parser and renderer tests before prompt changes.
4. Replace each unit's prompt contract from JSON to DSL.
5. Replace `serde_json::from_value` paths for migrated units with typed DSL parse results.
6. Preserve fallback values where they are deterministic and safe.
7. Remove or mark obsolete JSON repair calls for migrated units.
8. Update tests and docs after each unit family is migrated.

## Verification

Required commands:

```bash
cargo fmt --check
cargo test intel_units
cargo test routing
cargo test orchestration_helpers
cargo test dsl
cargo check --all-targets
```

Required coverage:

- router/classifier DSL parse and validation
- scope/list DSL parse and validation
- reviewer/verdict DSL parse and validation
- malformed DSL retry feedback
- fallback behavior for parser failure where fallback is intentionally safe
- no migrated unit still says `Return ONLY valid JSON`

## Done Criteria

- All live intel units stop asking models for JSON.
- Rust domain types remain typed and validated after DSL parsing.
- Retry and fallback behavior is at least as robust as the old JSON repair path.

## Anti-Patterns

- Do not replace JSON with verbose TOML/RON for model outputs.
- Do not use a single oversized universal grammar.
- Do not preserve JSON repair as a hidden dependency for migrated units.
