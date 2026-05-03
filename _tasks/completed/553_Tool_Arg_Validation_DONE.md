# 004 — Implement Strict Tool Argument JSON Validation

- **Priority**: Critical
- **Category**: Validation
- **Depends on**: 020 (defensive JSON parsing)
- **Blocks**: 008, 016

## Problem Statement

Tool call arguments are parsed from the model's JSON output using `serde_json::from_str()` and then accessed via ad-hoc field access like `av["command"].as_str().unwrap_or("")`. There is no:

1. **Schema validation**: No check that required fields exist before accessing them
2. **Type coercion**: No handling of near-miss types (e.g., model sends `"depth": 2` as string `"2"`)
3. **Field whitelist**: Extra/unexpected fields are silently ignored, potentially hiding model confusion
4. **Range validation**: Numeric fields like `depth`, `limit`, `offset` have no bounds checking
5. **Path traversal prevention**: Path fields aren't checked for `../` or absolute paths before every tool
6. **Consistent error messages**: Each tool returns different error format

Currently, model hallucination in tool arguments reaches the executor directly. For example, if a small model sends `{"path": "/etc/passwd"}` to the `read` tool, the executor catches the absolute path but only because of an ad-hoc check — not because of a systematic validation layer.

## Why This Matters for Small Local LLMs

Small models frequently produce:
- Malformed JSON (extra commas, unquoted strings) → currently caught by parse error
- JSON with wrong types (`"depth": "3"` instead of `3`) → silently coerced to 0/default
- JSON missing required fields → silently uses empty defaults
- JSON with hallucinated fields (`"mode": "overwrite"` on a read tool) → silently ignored
- Path traversal attempts from confused models → inconsistently caught

A systematic validation layer would catch these before execution and provide the model with structured, actionable error messages.

## Current Behavior

```rust
// tool_calling.rs - exec_ls
let depth = av["depth"].as_i64().unwrap_or(2).clamp(1, 5) as usize;
let ignore_patterns: Vec<String> = av["ignore"]
    .as_array()
    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
    .unwrap_or_default();
```

No validation that:
- `depth` is within bounds (ok, clamped here but not everywhere)
- `path` field doesn't contain `../`
- No extra unexpected fields are present
- The combination of fields is valid

## Recommended Target Behavior

Define a validation DSL/schema for each tool:

```rust
// Example schema for `ls` tool
ToolArgSchema::new("ls")
    .required("path", ArgType::RelPath)      // workspace-relative path, no ..
    .optional("depth", ArgType::UIntRange(1, 5), 2)  // default 2, range 1-5
    .optional("ignore", ArgType::ArrayOf(ArgType::String), vec![])
    .no_extra_fields()  // reject unknown fields
```

When validation fails, return a structured error:
```json
{
  "ok": false,
  "error": "validation_failed",
  "field_errors": [
    {"field": "path", "error": "absolute_path_not_allowed", "value": "/etc/passwd"}
  ]
}
```

## Source Files That Need Modification

- `src/tools/validation.rs` (new) — Schema definition and validation engine
- `src/tool_calling.rs` or new `src/tools/` directory — Integrate validation into `execute_tool_call`
- `src/tool_registry.rs` — Optionally attach schema to `ToolDefinitionExt`

## New Files/Modules

- `src/tools/validation.rs` — `ToolArgSchema`, `ValidationResult`, `FieldError`, `ArgType` enum

## Step-by-Step Implementation Plan

1. Define `ArgType` enum with variants:
   - `RelPath` — workspace-relative, no `..`, no `/` prefix
   - `RelPathOrGlob` — as above but allows `*`, `?`
   - `String_` — any non-empty string
   - `OptionalString` — string or null
   - `UInt` — positive integer
   - `UIntRange(min, max)` — bounded positive integer
   - `Bool` — boolean
   - `ArrayOf(Box<ArgType>)` — array of typed elements
   - `JsonValue` — arbitrary valid JSON
   - `Command` — shell command string (basic safety check)
   - `FileContent` — text content (size-bounded)

2. Define `ToolArgSchema`:
   ```rust
   struct ToolArgSchema {
       tool_name: String,
       required: Vec<(String, ArgType)>,
       optional: Vec<(String, ArgType, serde_json::Value)>, // name, type, default
       allow_extra_fields: bool,
   }
   ```

3. Implement `validate(schema: &ToolArgSchema, args: &serde_json::Value) -> ValidationResult`

4. Create schemas for each tool (initially the 12 most critical):
   - `shell`: `command` (required), `description` (optional)
   - `read`: `path` or `paths` (one required), `offset`, `limit` (optional)
   - `edit`: `path`, `old_string`, `new_string` (all required)
   - `write`: `path`, `content` (both required)
   - `search`: `pattern` (required), `path` (optional)
   - `glob`: `pattern` (required), `path` (optional)
   - `ls`: `path` (optional), `depth` (optional, 1-5)
   - `patch`: `patch` (required)
   - `stat`: `path` (required)
   - `copy`, `move`: `source`, `destination` (required)
   - `mkdir`: `path` (required)
   - `trash`: `path` (required)

5. Integrate validation into `execute_tool_call` BEFORE the tool match:
   ```rust
   if let Some(schema) = get_tool_schema(&tool_name) {
       if let Err(validation_error) = validate(schema, &args_value) {
           return validation_error.to_execution_result(call_id, tool_name);
       }
   }
   ```

6. Gradually add schemas for remaining tools

7. Add tests for each schema

## Recommended Crates

- `jsonschema` — optional, for JSON Schema-based validation if desired
- `camino` — for UTF-8 path handling in validation

## Validation/Sanitization Strategy

- Path fields: reject absolute paths, paths with `..`, paths outside workspace
- Command fields: basic injection check (no backticks, no `$()`, no `; rm`)
- Content fields: size cap (e.g., 1MB for write)
- All string fields: length bounds
- All numeric fields: range bounds

## Testing Plan

1. Unit test each schema: valid input passes, invalid input fails
2. Test edge cases: empty strings, unicode, very long values, null, missing fields, extra fields
3. Test path traversal attempts: `../`, `/etc/passwd`, `~/.ssh`
4. Test type coercion: `"3"` for uint field should fail, not silently default
5. Property-based tests with `proptest` for random valid/invalid inputs
6. Integration test: malformed model output → validation catches it → model receives structured error

## Acceptance Criteria

- All 30+ tools have arg schemas (or explicit `allow_any_args` marker)
- Path traversal attempts are caught by validation, not by executor
- Required field absence returns structured error with field name
- Type mismatches return structured error with expected vs actual type
- Validation happens BEFORE any I/O or execution
- No regression in existing tool behavior for valid inputs

## Risks and Migration Notes

- **Performance risk**: JSON schema validation on every tool call may add latency. Mitigate by making validation synchronous and fast (no allocations for simple checks).
- **Compatibility risk**: Strict `no_extra_fields` may break valid model outputs that include extra metadata. Consider making this configurable per-tool.
- **Incremental rollout**: Start with `read`, `write`, `edit`, `shell` schemas; expand to all tools over time.
