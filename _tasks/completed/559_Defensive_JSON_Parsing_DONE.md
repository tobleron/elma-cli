# 559 — Implement Defensive JSON Parsing with Repair for All Model Outputs

- **Priority**: Critical
- **Category**: Parsing
- **Depends on**: None
- **Blocks**: 553, 561

## Problem Statement

The codebase uses `serde_json::from_str()` or `serde_json::from_value()` at approximately 40+ call sites where model-generated JSON is parsed. When parsing fails:

1. **Most sites silently fall back to defaults** (e.g., `unwrap_or_default()`, `unwrap_or_else(...)`)
2. **Tool argument parsing** returns a string error but doesn't attempt repair (lines 62-85 in `tool_calling.rs`)
3. **Intel unit output parsing** often has retry logic but no structural repair attempt
4. **JSON from small models frequently has minor issues**: trailing commas, unquoted property names, single-quote strings, missing closing braces

The existing `json_repair.rs` module exists but is only called in specific paths (`chat_json_with_repair_timeout`), not universally. The `jsonrepair-rs` crate is listed as a dependency (line 54 of `Cargo.toml`) but may not be integrated at all call sites.

## Why This Matters for Small Local LLMs

Small models (4B params) produce malformed JSON at much higher rates than large models. Common failures:
- **Trailing commas**: `{"path": "src/main.rs",}` — ∼30% of failures
- **Unquoted keys**: `{path: "src/main.rs"}` — ∼20%
- **Single quotes**: `{'path': 'src/main.rs'}` — ∼15%
- **Missing braces**: `{"path": "src/main.rs"` — ∼10%
- **Extra text before/after JSON**: `Here's the result: {...}` — ∼10%
- **Wrong types**: `"depth": "3"` (string instead of number) — ∼15%

Without repair, every one of these failures results in loss of the model's intent, requiring a full retry that burns context window and adds latency.

## Current Behavior

```rust
// tool_calling.rs:62-86 — no repair attempt
let args_value: serde_json::Value = match serde_json::from_str(&tool_call.function.arguments) {
    Ok(v) => v,
    Err(e) => {
        // Just return error — model has to retry from scratch
        return ToolExecutionResult { ok: false, ... }
    }
};

// intel_narrative_steps.rs — uses unwrap_or_default() (silent failure)
let complexity: ComplexityAssessment = serde_json::from_value(output.data.clone())
    .unwrap_or_else(|_| ComplexityAssessment::default());
```

## Recommended Target Behavior

Create a universal `parse_model_json<T: DeserializeOwned>(raw: &str) -> Result<T, JsonParseError>` function that:

1. **Attempts strict parse** with `serde_json::from_str()`
2. **On failure, attempts repair**: Runs through deterministic repair pipeline
3. **On repair success, attempts parse again**
4. **On failure, extracts JSON from surrounding text** (model output wrapped in prose)
5. **Returns structured error** with: original text, repair steps attempted, final error, suggested fix for model

### Repair Pipeline (in order of attempt):
1. Strip markdown code fences (` ```json ... ``` `)
2. Strip surrounding prose (find first `{` to last `}`)
3. Fix trailing commas (regex: `,\s*}` → `}`)
4. Quote unquoted keys (heuristic: `word:` → `"word":`)
5. Fix single quotes to double quotes
6. Balance braces/brackets
7. Use `jsonrepair-rs` crate as final fallback

## Source Files That Need Modification

- `src/json_repair.rs` — Expand to universal repair pipeline
- `src/json_parser.rs` — Add unified `parse_model_json()` entry point
- `src/json_parser_extract.rs` — Add prose stripping, fence removal
- `src/tool_calling.rs` — Use `parse_model_json` for tool arguments
- `src/tool_loop.rs` — Use `parse_model_json` for tool call parsing
- `src/intel_trait.rs` — Use `parse_model_json` for intel unit outputs
- `src/orchestration_helpers/` — Use `parse_model_json` for all model JSON
- `src/routing_parse.rs` — Use `parse_model_json` for classification output
- All ∼40 `serde_json::from_str` / `from_value` call sites on model output

## New Files/Modules

- `src/json_repair_pipeline.rs` — Multi-step repair pipeline with metrics

## Step-by-Step Implementation Plan

1. Create `JsonRepairStep` enum with all repair strategies
2. Create `JsonParseError` with structured error info:
   ```rust
   pub struct JsonParseError {
       pub original: String,
       pub steps_attempted: Vec<String>,
       pub final_error: String,
       pub position: Option<(usize, usize)>, // line, col
       pub model_guidance: String,
   }
   ```
3. Implement `parse_model_json<T>()` in `json_parser.rs`
4. Add `repair_json_str(raw: &str) -> Option<String>` to `json_repair.rs`
5. Integrate `jsonrepair-rs` as final fallback in the repair pipeline
6. Update all tool argument parsing sites to use `parse_model_json`
7. Update all intel unit output parsing sites
8. Add structured error injection into conversation context
9. Add metrics tracking: repair success rate, failure reasons
10. Run full test suite

## Recommended Crates

- `jsonrepair-rs` — already a dependency (Cargo.toml:54), verify it's actually used
- `serde_json` — already used; leverage `from_str` with custom `Deserializer` for lenient mode
- `regex` — already a dependency; use for pattern-based repairs

## Validation/Sanitization Strategy

- Repair is deterministic (no model calls during repair)
- Each repair step logs what it changed for traceability
- Original model output is preserved in trace log for debugging
- Repaired output is flagged in evidence ledger

## Testing Plan

1. **Fuzzing tests**: Generate valid JSON, corrupt it with common errors (trailing comma, unquoted keys, etc.), verify repair succeeds
2. **Regression tests**: Collect real malformed outputs from small model runs, verify they parse after repair
3. **Roundtrip tests**: Valid JSON → repair → parse → should produce same value
4. **False positive tests**: Non-JSON text should fail cleanly, not produce garbage
5. **Performance tests**: Repair pipeline should complete in <1ms for typical tool arguments
6. **Edge cases**: Unicode, very long strings, nested objects, arrays, escaped quotes

## Acceptance Criteria

- All ∼40 model JSON parse sites use `parse_model_json()` instead of raw `serde_json::from_str()`
- Common small-model JSON errors (trailing commas, unquoted keys, single quotes, missing braces) are repaired
- Repair pipeline is deterministic and logged
- Existing valid JSON parsing behavior is unchanged
- New `json_fuzzing` tests pass with corrupted inputs

## Risks and Migration Notes

- **Repair altering semantics**: A repair that adds a missing closing brace at the wrong position could change the meaning. Always prefer the most conservative repair (closest brace match). Flag repaired JSON for model awareness.
- **Performance**: Multi-step repair on every tool argument parse may add latency. Benchmark before/after. Consider caching repair strategies per model type.
- **Dependency**: Verify `jsonrepair-rs` 0.1.0 (specified in Cargo.toml) is functional. If not, implement repairs inline.
- **Migration strategy**: First add `parse_model_json()` alongside existing `serde_json::from_str()` calls, then switch call sites one by one.
