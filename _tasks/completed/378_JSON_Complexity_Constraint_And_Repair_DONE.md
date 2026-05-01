# Task 378: JSON Format Complexity Constraint And Deterministic Repair Module

**Status:** Pending
**Priority:** HIGHEST (foundational for all intel unit reliability)
**Estimated effort:** 3-4 days
**Dependencies:** None (self-contained module)
**References:** objectives.md principles 2, 3, 4; AGENTS.md Rules 4, 7

## Problem

Small models (3B-4B) frequently produce malformed JSON for intel unit outputs. The current system:
1. Accepts deeply nested schemas (e.g., `workflow_schema` has 9 top-level fields + nested `scope` object with 7 sub-fields = 16 effective fields, 2 nesting levels)
2. Has no hard constraint on JSON complexity — schemas can grow unbounded
3. Uses `jsonrepair-rs` for repair but has no dedicated deterministic module for post-repair retry with a specialized intel unit
4. `regex_fallback_value` is a brute-force per-field parser, not a structured repair approach

Per objectives.md: "One intel unit = one narrow decision" and "If a step fails, it is split into smaller steps." The JSON output format must be so simple that a 3B model can reliably produce it.

## Objective

1. **Complexity constraint**: No intel unit output schema may have more than 1 nested object level (2-level max), and no more than 3 required fields by default. Five total fields is the absolute maximum, including optional fields. Any need for more than five pieces of information must be moved into another focused intel unit.

2. **Deterministic repair module**: Create a dedicated `src/json_repair.rs` module that handles all JSON parse failures deterministically before any LLM-based retry. The repair pipeline is:
   - Parse → Fail → Try `jsonrepair-rs` → Fail → Regex field extraction → Fail → Specialized LLM retry with simplified prompt

3. **Specialized repair intel unit**: A focused intel unit (`JsonRepairUnit`) that takes the raw model output and a minimal schema definition (only required field names and types) and asks the model to output valid JSON matching that schema — nothing more.

## Rules (Non-Negotiable)

| Rule | Value | Rationale |
|------|-------|-----------|
| Max nesting depth | 2 levels | `{key: {sub_key: value}}` is max. No `{a: {b: {c: value}}}` |
| Max required fields | 3 | Small models reliably produce 3-field JSON |
| Max total fields | 5 (absolute) | Includes optional fields. Beyond 5, splitting is required |
| Default field count | 3 | New intel units should target 1-3 fields unless a reviewed exception is justified |
| No arrays of objects | Banned | `["a", "b"]` OK. `[{...}, {...}]` NOT OK |
| No arrays of arrays | Banned | Only 1D arrays allowed |
| Schema enforcement | Compile-time | `schema_for_type<T>()` must verify these constraints at registration |

## Implementation Plan

### Phase 1: Schema Complexity Validation

Add a `validate_schema_complexity()` function to `src/json_error_handler/schemas.rs`:

```rust
const MAX_NESTING_DEPTH: usize = 2;
const MAX_REQUIRED_FIELDS: usize = 3;
const MAX_TOTAL_FIELDS: usize = 5;

fn validate_schema_complexity(schema: &JsonSchema, type_name: &str) -> Result<()> {
    // Check required field count
    if schema.required_fields.len() > MAX_REQUIRED_FIELDS {
        bail!("Schema for '{}' has {} required fields (max {})",
              type_name, schema.required_fields.len(), MAX_REQUIRED_FIELDS);
    }

    // Check total field count (required + optional via field_types)
    if schema.field_types.len() > MAX_TOTAL_FIELDS {
        bail!("Schema for '{}' has {} total fields (max {})",
              type_name, schema.field_types.len(), MAX_TOTAL_FIELDS);
    }

    // Check nesting depth
    let depth = max_nesting_depth(&schema);
    if depth > MAX_NESTING_DEPTH {
        bail!("Schema for '{}' has nesting depth {} (max {})",
              type_name, depth, MAX_NESTING_DEPTH);
    }

    // Check for arrays of objects
    if has_object_arrays(&schema) {
        bail!("Schema for '{}' contains arrays of objects (banned)", type_name);
    }

    Ok(())
}
```

Integrate into `schema_for_type()` so violations are caught at startup (panic or log).

### Phase 2: Decompose Violating Schemas

Current schemas that violate the new rules:

**workflow_schema** (9 required + 7 scope sub-fields = 16 total, 2-level nesting):
- Split into 3 separate intel unit outputs:
  1. `workflow_basic` → `{objective, complexity, risk}` (3 required, 3 total)
  2. `workflow_scope` → `{focus_paths, include_globs, exclude_globs}` (3 required, 3 total) — split from scope
  3. `workflow_formula` → `{preferred_formula, reason}` (2 required, 2 total)

**complexity_schema** (7 total fields):
- Split:
  1. `complexity_core` → `{complexity, risk, needs_evidence}` (3 required, 3 total)
  2. `complexity_capabilities` → `{needs_tools, needs_decision, needs_plan}` (3 required, 3 total)

**scope_schema** (7 total fields):
- Split:
  1. `scope_core` → `{objective, reason}` (2 required, 2 total)
  2. `scope_paths` → `{focus_paths, include_globs, exclude_globs}` (3 required, 3 total)

**ClaimCheckVerdict** (5 total — borderline, keep as-is but mark as MAX):
- No split needed, but no more fields can be added

Any future schema that wants more than five fields must be rejected during review and implemented as another intel unit. The system must decompose cognitive jobs instead of increasing JSON shape complexity.

### Phase 3: Dedicated JSON Repair Module

Create `src/json_repair.rs`:

```rust
//! Deterministic JSON repair pipeline for intel unit outputs.
//!
//! Repair stages (in order, stop on first success):
//! 1. Direct parse (serde_json::from_str)
//! 2. jsonrepair-rs (fix trailing commas, missing quotes, etc.)
//! 3. Regex field extraction (regex_fallback_value for known types)
//! 4. LLM-based repair (JsonRepairUnit — specialized intel unit)

pub(crate) enum RepairStage {
    DirectParse,
    JsonRepairRs,
    RegexExtraction,
    LlmRepair,
    Failed,
}

pub(crate) struct RepairResult<T> {
    pub(crate) value: T,
    pub(crate) stage: RepairStage,
    pub(crate) original_error: Option<String>,
}

/// Deterministic repair pipeline: tries each stage, stops on first success.
/// LLM repair (stage 4) is only invoked if stages 1-3 fail.
pub(crate) async fn repair_json<T: DeserializeOwned + 'static>(
    raw: &str,
    client: &reqwest::Client,
    chat_url: &Url,
    repair_profile: &Profile,
) -> Result<RepairResult<T>> {
    // Stage 1: Direct parse
    if let Ok(value) = serde_json::from_str::<T>(raw) {
        return Ok(RepairResult { value, stage: RepairStage::DirectParse, original_error: None });
    }

    // Stage 2: jsonrepair-rs
    if let Ok(repaired_str) = jsonrepair(raw) {
        if let Ok(value) = serde_json::from_str::<T>(&repaired_str) {
            return Ok(RepairResult { value, stage: RepairStage::JsonRepairRs, original_error: None });
        }
    }

    // Stage 3: Regex field extraction (existing regex_fallback_value)
    if let Some(fallback_value) = regex_fallback_value::<T>(raw) {
        if let Ok(value) = serde_json::from_value(fallback_value) {
            return Ok(RepairResult { value, stage: RepairStage::RegexExtraction, original_error: None });
        }
    }

    // Stage 4: LLM repair via specialized intel unit
    // (implementation in Phase 4)
    ...
}
```

### Phase 4: Specialized LLM Repair Intel Unit

Create `src/intel_units/intel_units_json_repair.rs`:

A focused intel unit that:
- Takes ONLY: raw model output + a minimal schema (field names + types, NO nested objects)
- Uses a strict system prompt: "Output valid JSON with exactly these fields. No markdown. No extra text."
- Returns simple JSON output
- Has a short timeout (30s) and 1 retry max

The schema passed to the repair unit must be the DECOMPOSED version — never a complex nested schema. If the original output came from a complex schema, the repair unit treats it one decomposed field at a time.

### Phase 5: Wire Into All Intel Unit Executions

Modify the intel unit execution pipeline (`src/intel_trait.rs` or the centralized executor) so that:
1. Every intel unit output goes through `repair_json` before being returned
2. Parse failures are logged with the stage that succeeded
3. If all 4 stages fail, the intel unit returns an error (no silent corruption)

## Files to Create/Modify

| File | Action |
|------|--------|
| `src/json_repair.rs` | CREATE — Deterministic 4-stage repair pipeline |
| `src/intel_units/intel_units_json_repair.rs` | CREATE — Specialized LLM repair intel unit |
| `src/json_error_handler/schemas.rs` | MODIFY — Add `validate_schema_complexity()`, decompose violating schemas |
| `src/intel_units/intel_units_core.rs` | MODIFY — Split complex intel units into decomposed units |
| `src/json_parser.rs` | MODIFY — Integrate repair pipeline into `parse_with_repair` |
| `src/defaults_evidence.rs` | MODIFY — Add `json_repair` profile config |
| `_tasks/pending/` | Possible sub-tasks for each decomposed schema |

## Non-Scope

- Do NOT modify `src/prompt_core.rs`
- Do NOT change the format of non-JSON intel unit outputs (legacy token format is fine for simple classification)
- Do NOT add new dependencies beyond existing crates (`serde_json`, `jsonrepair-rs`)

## Risk Assessment

- **HIGH**: Splitting complex schemas requires corresponding intel unit code changes. Each split schema needs its own intel unit call (increases total LLM calls but each is more reliable)
- **MEDIUM**: Existing tests may break when schemas are decomposed. Update test expectations.
- **LOW**: The repair pipeline is additive — direct parse and jsonrepair-rs stages are unchanged, only LLM stage 4 is new

## Verification

```bash
cargo build
cargo test json_repair
cargo test schema
cargo test intel_unit
cargo test parse
```

**Schema audit**: Run `cargo test schema_complexity` (new test that calls `validate_schema_complexity` on every registered schema) and verify no violations.

**Repair stage coverage**: Test each of the 4 repair stages with deliberately malformed JSON:
- Stage 1: Valid JSON → direct parse
- Stage 2: Missing closing brace → jsonrepair-rs fixes
- Stage 3: Natural language text wrapping JSON → regex extraction
- Stage 4: Completely garbled output → LLM repair reconstructs from field names
