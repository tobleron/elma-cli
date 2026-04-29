# Task 384: DSL Certification Harness And Dead JSON Output Removal

**Status:** pending
**Priority:** critical
**Suite:** Compact DSL Model-Output Migration
**Depends on:** Tasks 376-383
**Blocks:** DSL protocol/skill certification tasks 364-375 after reframe

## Objective

Certify that Elma no longer asks models to emit JSON for structured outputs, then remove obsolete model-output JSON parsers, repair units, prompts, tests, and documentation. This is the release gate for the compact DSL migration.

## Required Deliverables

- `_scripts/certify_dsl_migration.sh`
- `docs/dsl/CERTIFICATION_REPORT.md`
- Updated DSL protocol/self-test prompt packs.
- Static grep/assertion gate for active model-output JSON prompts.
- Removal or quarantine of dead JSON-output code.

## Certification Dimensions

The report must cover:

- action DSL parser/executor
- every migrated intel DSL family
- prompt/profile contracts
- GBNF grammar availability and parser fallback
- repair loop behavior
- stop-policy behavior
- transcript/session visibility
- remaining intentional JSON boundaries
- removed or intentionally retained JSON code paths

## Static Gate Rules

Fail the certification if active code/config prompts contain unapproved model-output JSON contracts, including:

- `Return ONLY valid JSON`
- `Output valid JSON only`
- `Program JSON`
- JSON schemas embedded in profile prompts for model output
- `chat_json_with_repair` on a migrated path

Allow only documented exceptions from `docs/dsl/JSON_BOUNDARIES.md`.

## Implementation Steps

1. Add static certification script and allowlist file for intentional boundaries.
2. Add DSL parser/executor certification tests.
3. Add prompt-pack tests for action DSL and priority intel DSLs.
4. Remove dead JSON grammar mappings and model-output JSON profile text.
5. Remove or quarantine obsolete JSON repair modules no longer called by live paths.
6. Update docs from "tool calling" to "DSL action protocol" where behavior changed.
7. Run the full certification and publish the report.

## Verification

Required commands:

```bash
bash -n _scripts/certify_dsl_migration.sh
_scripts/certify_dsl_migration.sh
cargo fmt --check
cargo test dsl
cargo test agent_protocol
cargo test intel_units
cargo test tool_loop
cargo test stop_policy
cargo test prompt_core
cargo test
cargo check --all-targets
```

Required checks:

- no active migrated prompt asks for JSON
- invalid DSL repair tests pass
- all intentional JSON boundaries are documented
- old live model-output JSON repair paths are removed or unreachable
- DSL prompt tests pass against the local harness or documented manual run

## Done Criteria

- Elma's active model-output structured protocol is compact DSL.
- Remaining JSON is internal/provider/config-boundary only and documented.
- The repository has a repeatable gate preventing JSON-output regression.

## Anti-Patterns

- Do not leave old JSON repair code active because it is convenient.
- Do not claim migration complete while any live model-output JSON prompt remains.
- Do not hide failed certification cases in trace-only logs.
