# Task 382: DSL GBNF Grammar Family And Profile Migration

**Status:** pending
**Priority:** critical
**Suite:** Compact DSL Model-Output Migration
**Depends on:** Tasks 376, 377
**Blocks:** Tasks 378, 380, 384

## Objective

Add optional constrained-decoding grammars for the compact DSL family and migrate model profiles/prompts away from JSON-output contracts. Grammar support improves output shape, while Rust parsing and validation remain authoritative.

## Prompt-Core Approval

The user explicitly approved changing `src/prompt_core.rs` for this migration. Any prompt-core change must still:

- update the protected prompt hash
- run prompt hash tests
- run scenario/regression tests
- document the change in the task report

## Required Deliverables

- GBNF grammar files under `config/grammars/` for action DSL and priority intel DSLs.
- Updated `config/grammar_mapping.toml` where profile-level grammar injection still applies.
- Updated default/model profile prompts that no longer instruct model-produced JSON.
- Updated `src/prompt_core.rs` DSL contract and hash.
- `docs/dsl/GRAMMARS.md`

## Grammar Strategy

- Prefer small grammars per DSL family.
- Keep grammars intentionally boring: no nested structures, no arbitrary sublanguages.
- Use grammar to prevent obvious garbage, not to replace parser/validator safety.
- If llama.cpp grammar support is unavailable or rejected, continue with parser-only mode and log a visible degraded capability.

## Implementation Steps

1. Define grammar text for the action DSL.
2. Define grammar text for one-token verdict and compact key/value DSLs.
3. Add grammar validation tests using existing grammar infrastructure.
4. Update profile prompts from JSON to DSL one family at a time.
5. Update prompt-core tool workflow to describe DSL actions instead of native tool calls.
6. Update prompt hash after review/validation.
7. Remove obsolete JSON grammar mappings for migrated model-output profiles.
8. Keep provider-wire JSON request/response handling unchanged.

## Verification

Required commands:

```bash
cargo fmt --check
cargo test grammar
cargo test prompt_core
cargo test dsl
cargo test intel_units
cargo check --all-targets
```

Required checks:

- action grammar validates
- intel DSL grammars validate
- profile grammar mappings point to existing files
- prompt-core hash test passes after intentional update
- migrated prompts do not say `Return ONLY valid JSON`
- grammar-disabled path still parses/validates DSL deterministically

## Done Criteria

- Models are instructed to emit DSL, not JSON, on migrated paths.
- Grammar is optional defense-in-depth, not the source of trust.
- Prompt-core protection remains meaningful after the approved update.

## Anti-Patterns

- Do not make GBNF the only validation layer.
- Do not keep JSON grammars attached to profiles whose prompts now require DSL.
- Do not add long example-heavy prompts to compensate for parser mistakes.
