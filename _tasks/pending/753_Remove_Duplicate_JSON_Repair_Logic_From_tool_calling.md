# Task 753: Remove Duplicate JSON Repair Logic From tool_calling.rs

## Type

Architecture / Reliability / Code Quality

## Severity

High

## Scope

Tool calling, JSON parsing, error handling

## Problem

`src/tool_calling.rs` contains its own JSON repair and parsing logic that duplicates dedicated modules:

```rust
// Lines 135-177 in tool_calling.rs
let args_value: serde_json::Value = match serde_json::from_str(&tool_calling.function.arguments) {
    Ok(v) => v,
    Err(_first_err) => {
        let raw = &tool_calling.function.arguments;
        match crate::json_parser::parse_model_json::<serde_json::Value>(raw) {
            Ok(v) => { ... }
            Err(_) => {
                // Returns ToolExecutionResult with parse error
            }
        }
    }
};
```

This is followed by:
```rust
// Lines 182-200: manual arg repair for read/exists tools
let args_value = {
    let needs_repair = if tool_name == "read" { ... }
    else if tool_name == "exists" { ... };
    if needs_repair {
        let raw = &tool_calling.function.arguments;
        let path_re = regex::Regex::new(r#"["']?([^"']+)["']?"#).unwrap();
        // ... manual regex extraction ...
    }
};
```

The codebase already has:
- `src/json_parser.rs` — robust JSON parsing with repair
- `src/json_repair.rs` — deterministic JSON repair pipeline
- `src/json_error_handler/` — JSON error handling with circuit breaker
- `src/strict_tool_parser.rs` — strict tool argument parsing (Task 645)

The duplication in `tool_calling.rs`:
1. **Violates DRY** — repair logic lives in two places
2. **Hides errors** — if `json_parser.rs` is updated, `tool_calling.rs` won't benefit
3. **Brittle** — the manual regex repair for `read`/`exists` is a workaround, not a fix
4. **Prevents testing** — the repair logic in `tool_calling.rs` is embedded in a large async function, making it hard to unit test
5. **Contradicts Rule 7** — "Better evidence, better narrative, narrower intel decomposition" is preferred over "more heuristics"

The manual regex repair (lines 182-200) is especially problematic. It's a symptom of the model not understanding tool schemas, but the fix is applied as a band-aid in the executor rather than improving the schema or parser.

## Root Cause

`tool_calling.rs` was written before the dedicated JSON modules were mature. The repair logic was added incrementally as a quick fix and never refactored out.

## Proposed Solution

### Phase 1 — Replace inline parsing with canonical pipeline

1. In `tool_calling.rs`, replace the inline `serde_json::from_str` + `json_parser::parse_model_json` block with a single call to `json_parser::parse_tool_arguments(tool_name, raw_args)`
2. Create `parse_tool_arguments()` in `src/json_parser.rs` (or `src/strict_tool_parser.rs`) that:
   - Attempts strict JSON parse first
   - Falls back to `json_repair.rs` pipeline
   - Falls back to schema-aware extraction (if strict parser exists)
   - Returns a structured error, not just a string

### Phase 2 — Delete manual arg repair

1. Move the current `read`/`exists` recovery cases into canonical parser tests before deleting the inline path.
2. Delete the manual regex repair for `read`/`exists` in `tool_calling.rs` only after the canonical parser passes those cases.
3. Move any genuinely needed repair logic to `src/tool_repair.rs` (Task 689: schema-guided tool argument repair)
4. `tool_repair.rs` should be the single canonical location for schema-aware argument repair
5. If `tool_repair.rs` doesn't exist yet, the repair logic should be added to `json_parser.rs` or `strict_tool_parser.rs`

### Phase 3 — Unify error contracts

1. The JSON parser modules should return a structured error that `tool_calling.rs` can convert into a `ToolExecutionResult`
2. Error messages should follow the model-facing error contract (Task 645)
3. Error messages should include the correct schema template, not just "check the arguments"

### Phase 4 — Verify no regression

1. Ensure that malformed `read`/`exists` calls still get repaired, but via the canonical parser
2. Ensure that completely unparseable JSON still returns a clear error to the model
3. Ensure the repair doesn't hide schema problems that should be fixed at the prompt level

## Acceptance Criteria

- [ ] `tool_calling.rs` contains no inline JSON parsing logic
- [ ] `tool_calling.rs` contains no manual regex-based argument repair
- [ ] All JSON parsing goes through `json_parser.rs` or `strict_tool_parser.rs`
- [ ] Schema-aware repair lives in `tool_repair.rs` (or `strict_tool_parser.rs` if Task 689 not done)
- [ ] `format_tool_error_correction()` still provides useful schema templates
- [ ] `cargo build && cargo test` passes
- [ ] Unit tests for JSON parsing are in `json_parser.rs`, not `tool_calling.rs`

## Verification Plan

- `grep -n "serde_json::from_str" src/tool_calling.rs` → only in re-exports or canonical parser calls
- `grep -n "Regex::new" src/tool_calling.rs` → no matches for argument repair
- Unit test in `json_parser.rs`: malformed tool arguments are repaired correctly
- Unit test in `json_parser.rs`: unparseable arguments return structured error
- Integration test: model sends malformed `read` call → system repairs via canonical path

## Dependencies

- `src/json_parser.rs` (canonical parsing)
- `src/json_repair.rs` (repair pipeline)
- `src/strict_tool_parser.rs` (Task 645)
- `src/tool_repair.rs` (Task 689)

## Notes

The manual regex repair in `tool_calling.rs` is a classic example of "patching symptoms instead of fixing the system." The docs say:

> "Never respond to small-model weakness by stuffing more examples into prompts, overfitting rules, or bloating context."
> "Do: decompose, add a focused intermediary intel unit, tighten narrative context, or reduce cognitive load per step."

The correct fix is:
1. Improve the tool declaration/schema and parser contract outside `src/prompt_core.rs` unless the user explicitly approves a core prompt change.
2. Use `strict_tool_parser.rs` for schema validation (focused intermediary).
3. Return structured errors to the model so it can self-correct (decomposition).

Do not preserve the regex repair as a second executor-side fallback. Preserve its useful recovery behavior in the canonical parser, then delete the duplicate inline implementation.
