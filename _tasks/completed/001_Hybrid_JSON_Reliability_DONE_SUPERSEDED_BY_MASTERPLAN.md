# Task 001: Hybrid JSON Reliability (GBNF + Verification + Repair)

## Priority
**P0-1.1 - CRITICAL (FIRST TASK IN ROADMAP)**

## Status
**PENDING** → Ready to move to `_tasks/active/`

## Objective

Implement **defense-in-depth JSON reliability** with 5 layers:
1. **GBNF Grammar** (prevention at token generation)
2. **Few-Shot Examples** (format teaching in prompts)
3. **Auto-Repair Parser** (existing jsonrepair-rs + enhancements)
4. **Schema Validation** (required fields + type checking)
5. **Fallback Values** (safe defaults when all else fails)

**Target:** 99.9%+ JSON parse success rate with semantic validity.

---

## Background

### Current State (Verified)
- ✅ llama.cpp endpoint **supports GBNF grammars** (tested 2026-04-02)
- ✅ `jsonrepair-rs` already integrated in `src/json_parser.rs`
- ✅ Circuit breaker in `src/json_error_handler.rs`
- ✅ Multiple parse methods (JsonDirect, JsonMarkdown, JsonExtracted)
- ✅ 78 default profiles with JSON output requirements

### Problems to Solve
- ❌ No GBNF grammar enforcement (relies on model goodwill)
- ❌ Parse failure rate ~5-15% (unacceptable for production)
- ❌ No schema validation (missing fields not detected)
- ❌ No few-shot examples in prompts (model guesses format)
- ❌ Fallback values used too often (semantic drift)

---

## Technical Tasks

### Phase 0: Infrastructure Setup
- [ ] **Create grammar directory**
  - `config/grammars/` for GBNF grammar files
- [ ] **Update Profile struct**
  - Add `grammar_path: Option<String>` field
  - Add `few_shot_examples: Option<String>` field
- [ ] **Update chat_once()**
  - Load grammar from file if specified
  - Inject grammar into request
- [ ] **Create test harness**
  - Script to test grammar with each profile
  - Measure parse success rate before/after

**Files to Create:**
- `config/grammars/README.md` (grammar documentation)
- `src/json_grammar.rs` (grammar loading + injection)

**Files to Modify:**
- `src/types_core.rs` (add grammar_path to Profile)
- `src/models_api.rs` (add grammar to ChatCompletionRequest)
- `src/storage.rs` (load grammar files)

---

### Phase 1: GBNF Grammars for Critical Paths (4 profiles)

#### Grammar 1: Router (5-choice classification)
**File:** `config/grammars/router_choice_1of5.json.gbnf`

```bnf
root ::= "{" ws "\"choice\":" ws string ws "," ws "\"label\":" ws string ws "," ws "\"reason\":" ws string ws "," ws "\"entropy\":" ws number ws "}" ws
string ::= "\"" [a-zA-Z0-9_ ]* "\""
number ::= [0-9]+ "." [0-9]+
ws ::= [ \t\n]*
```

**Profile:** `config/defaults/router.toml`
- Add `grammar_path = "grammars/router_choice_1of5.json.gbnf"`
- Add 2-3 few-shot examples to `system_prompt`

#### Grammar 2: Speech Act (3-choice classification)
**File:** `config/grammars/speech_act_choice_1of3.json.gbnf`

```bnf
root ::= "{" ws "\"choice\":" ws act ws "," ws "\"label\":" ws act ws "," ws "\"reason\":" ws string ws "," ws "\"entropy\":" ws number ws "}" ws
act ::= "\"CHAT\"" | "\"INQUIRE\"" | "\"INSTRUCT\""
string ::= "\"" [a-zA-Z0-9_ ]* "\""
number ::= [0-9]+ "." [0-9]+
ws ::= [ \t\n]*
```

**Profile:** `config/defaults/speech_act.toml`

#### Grammar 3: Mode Router (4-choice classification)
**File:** `config/grammars/mode_router_choice_1of4.json.gbnf`

```bnf
root ::= "{" ws "\"choice\":" ws mode ws "," ws "\"label\":" ws mode ws "," ws "\"reason\":" ws string ws "," ws "\"entropy\":" ws number ws "}" ws
mode ::= "\"INSPECT\"" | "\"EXECUTE\"" | "\"PLAN\"" | "\"MASTERPLAN\""
string ::= "\"" [a-zA-Z0-9_ ]* "\""
number ::= [0-9]+ "." [0-9]+
ws ::= [ \t\n]*
```

**Profile:** `config/defaults/mode_router.toml`

#### Grammar 4: Complexity Assessor (4-choice classification)
**File:** `config/grammars/complexity_choice_1of4.json.gbnf`

```bnf
root ::= "{" ws "\"choice\":" ws complexity ws "," ws "\"label\":" ws complexity ws "," ws "\"reason\":" ws string ws "," ws "\"entropy\":" ws number ws "}" ws
complexity ::= "\"DIRECT\"" | "\"INVESTIGATE\"" | "\"MULTISTEP\"" | "\"OPEN_ENDED\""
string ::= "\"" [a-zA-Z0-9_ ]* "\""
number ::= [0-9]+ "." [0-9]+
ws ::= [ \t\n]*
```

**Profile:** `config/defaults/complexity_assessor.toml`

---

### Phase 2: Few-Shot Examples for All Critical Profiles

**Update system prompts to include examples:**

```toml
# config/defaults/router.toml
system_prompt = """
You are Elma's Route Classifier.

Return the most probable route based on the user message and workspace context.

Output format:
{"choice": "ROUTE", "label": "ROUTE", "reason": "one sentence", "entropy": 0.XX}

Examples:
{"choice": "CHAT", "label": "CHAT", "reason": "User asks conceptual question", "entropy": 0.12}
{"choice": "INVESTIGATE", "label": "INVESTIGATE", "reason": "Need to explore workspace first", "entropy": 0.34}
{"choice": "SHELL", "label": "SHELL", "reason": "User requests command execution", "entropy": 0.18}

Return ONLY the JSON object. No explanations.
"""
```

**Profiles to update:**
- [ ] router.toml
- [ ] speech_act.toml
- [ ] mode_router.toml
- [ ] complexity_assessor.toml
- [ ] workflow_planner.toml
- [ ] formula_selector.toml
- [ ] scope_builder.toml

---

### Phase 3: Schema Validation

**Create schema definitions:**

```rust
// src/json_error_handler.rs
pub struct JsonSchema {
    pub required_fields: Vec<&'static str>,
    pub field_types: HashMap<&'static str, FieldType>,
    pub validators: Vec<Box<dyn FieldValidator>>,
}

pub enum FieldType {
    String,
    Number,
    Choice(&'static [&'static str]),  // Enum validation
}

pub trait FieldValidator {
    fn validate(&self, field: &str, value: &serde_json::Value) -> ValidationResult;
}

// Example validators
pub struct EntropyValidator;  // 0.0 <= entropy <= 1.0
pub struct RequiredChoiceValidator;  // choice must be in allowed set
pub struct ReasonLengthValidator;  // reason must be 10-200 chars
```

**Implement validation:**

```rust
// src/json_parser.rs
pub fn validate_schema(json: &serde_json::Value, schema: &JsonSchema) -> ValidationResult {
    // Check required fields present
    for field in &schema.required_fields {
        if json.get(field).is_none() {
            return Err(format!("Missing required field: {}", field));
        }
    }
    
    // Check field types
    for (field, expected_type) in &schema.field_types {
        if let Some(value) = json.get(field) {
            match expected_type {
                FieldType::String if !value.is_string() => {
                    return Err(format!("Field {} must be string", field));
                }
                FieldType::Number if !value.is_number() => {
                    return Err(format!("Field {} must be number", field));
                }
                FieldType::Choice(allowed) => {
                    if let Some(s) = value.as_str() {
                        if !allowed.contains(&s) {
                            return Err(format!("Field {} must be one of: {:?}", field, allowed));
                        }
                    }
                }
                _ => {}
            }
        }
    }
    
    // Run custom validators
    for validator in &schema.validators {
        validator.validate(field, value)?;
    }
    
    Ok(())
}
```

**Schemas to create:**
- [ ] Router schema (choice, label, reason, entropy)
- [ ] Speech act schema (choice, label, reason, entropy)
- [ ] Mode router schema (choice, label, reason, entropy)
- [ ] Complexity schema (choice, label, reason, entropy)
- [ ] Workflow planner schema (objective, complexity, risk, reason, steps)
- [ ] Formula selector schema (formula, confidence, reason)

---

### Phase 4: Enhanced Auto-Repair

**Current `json_parser.rs` already has:**
- ✅ Markdown extraction
- ✅ JSON-from-text extraction
- ✅ jsonrepair-rs integration

**Enhancements to add:**

```rust
// src/json_parser.rs
pub fn parse_with_repair(raw: &str, schema: &JsonSchema) -> ParseResult {
    // Pass 1: Direct JSON parse
    if let Ok(json) = serde_json::from_str(raw) {
        if validate_schema(&json, schema).is_ok() {
            return ParseResult::Success(json, ParseMethod::JsonDirect);
        }
    }
    
    // Pass 2: Markdown extraction + parse
    if let Some(extracted) = extract_from_markdown(raw) {
        if let Ok(json) = serde_json::from_str(&extracted) {
            if validate_schema(&json, schema).is_ok() {
                return ParseResult::Success(json, ParseMethod::JsonMarkdown);
            }
        }
    }
    
    // Pass 3: JSON-from-text extraction + parse
    if let Some(extracted) = extract_json_from_text(raw) {
        if let Ok(json) = serde_json::from_str(&extracted) {
            if validate_schema(&json, schema).is_ok() {
                return ParseResult::Success(json, ParseMethod::JsonExtracted);
            }
        }
    }
    
    // Pass 4: jsonrepair-rs repair + parse
    if let Ok(repaired) = jsonrepair(raw) {
        if let Ok(json) = serde_json::from_str(&repaired) {
            if validate_schema(&json, schema).is_ok() {
                return ParseResult::Success(json, ParseMethod::JsonRepaired);
            }
        }
    }
    
    // Pass 5: Regex extraction for critical fields
    let fallback = extract_with_regex(raw, schema);
    if fallback.is_some() {
        return ParseResult::Success(fallback.unwrap(), ParseMethod::RegexFallback);
    }
    
    // Pass 6: All failed
    ParseResult::Failed("All parse methods failed".to_string())
}

fn extract_with_regex(raw: &str, schema: &JsonSchema) -> Option<serde_json::Value> {
    // Extract "choice": "VALUE" with regex
    // Extract "entropy": 0.XX with regex
    // Build minimal valid JSON object
    // Validate against schema
}
```

---

### Phase 5: Testing & Metrics

**Test harness script:**
```bash
#!/bin/bash
# _scripts/test_json_reliability.sh

echo "Testing JSON Reliability Layers"
echo "================================"

# Test 1: GBNF Grammar Enforcement
echo "Test 1: GBNF Grammar (router.toml)"
for i in {1..20}; do
    curl -X POST http://192.168.1.186:8080/completion \
      -H "Content-Type: application/json" \
      -d "{\"prompt\": \"Classify: $1\", \"grammar\": \"...\"}" \
      | jq -r '.content' \
      | python3 -c "import sys, json; json.loads(sys.stdin.read())" \
      && echo "PASS" || echo "FAIL"
done

# Test 2: Parse Success Rate (with repair)
echo "Test 2: Auto-Repair Parser"
# Run scenarios and measure parse failures

# Test 3: Schema Validation
echo "Test 3: Schema Validation"
# Inject malformed JSON and verify rejection

# Report metrics
echo "Metrics:"
echo "- Parse success rate: X%"
echo "- GBNF enforcement rate: X%"
echo "- Schema validation pass: X%"
echo "- Average latency: Xms"
```

**Metrics to track:**
- Parse success rate (target: >99.9%)
- GBNF grammar success rate (target: 100%)
- Schema validation pass rate (target: >98%)
- Average parse latency (target: <50ms)
- Fallback usage rate (target: <1%)

---

## Acceptance Criteria

- [ ] **GBNF grammars** created for 4 critical profiles
- [ ] **Few-shot examples** added to 7 critical profiles
- [ ] **Schema validation** implemented for all output types
- [ ] **Auto-repair parser** enhanced with regex fallback
- [ ] **Parse success rate** >99.9% (measured over 100 requests)
- [ ] **Latency** not increased by >10% vs. current
- [ ] **All scenario tests** passing
- [ ] **Metrics dashboard** created (parse failures, latency, fallback rate)

---

## Dependencies

- ✅ llama.cpp GBNF support verified (2026-04-02)
- ✅ jsonrepair-rs already in Cargo.toml
- ⏳ None (this is the first foundational task)

---

## Files to Create

| File | Purpose |
|------|---------|
| `config/grammars/README.md` | Grammar documentation |
| `config/grammars/router_choice_1of5.json.gbnf` | Router grammar |
| `config/grammars/speech_act_choice_1of3.json.gbnf` | Speech act grammar |
| `config/grammars/mode_router_choice_1of4.json.gbnf` | Mode router grammar |
| `config/grammars/complexity_choice_1of4.json.gbnf` | Complexity grammar |
| `src/json_grammar.rs` | Grammar loading + injection |
| `_scripts/test_json_reliability.sh` | Test harness |

---

## Files to Modify

| File | Change |
|------|--------|
| `src/types_core.rs` | Add `grammar_path` to Profile |
| `src/models_api.rs` | Add `grammar` to ChatCompletionRequest |
| `src/storage.rs` | Load grammar files |
| `src/json_parser.rs` | Add schema validation, regex fallback |
| `src/json_error_handler.rs` | Add schema definitions |
| `config/defaults/router.toml` | Add grammar_path + examples |
| `config/defaults/speech_act.toml` | Add grammar_path + examples |
| `config/defaults/mode_router.toml` | Add grammar_path + examples |
| `config/defaults/complexity_assessor.toml` | Add grammar_path + examples |

---

## Verification

1. **Run intention scenarios:**
   ```bash
   ./run_intention_scenarios.sh
   # Verify 100% parse success
   ```

2. **Stress test with malformed outputs:**
   ```bash
   # Inject malformed JSON and verify repair
   ```

3. **Measure before/after metrics:**
   ```bash
   # Parse success rate: ~85-90% → >99.9%
   # Latency: baseline → +5-10% (acceptable)
   ```

---

## Notes

**GBNF Grammar Syntax:**
- `root ::= ...` defines the root rule
- `string ::= "\"" [a-zA-Z0-9_ ]* "\""` matches quoted strings
- `number ::= [0-9]+ "." [0-9]+` matches decimals
- `ws ::= [ \t\n]*` matches whitespace
- `|` denotes choice (OR)
- Concatenation denotes sequence (AND)

**Testing Tip:**
Test grammars with curl first, then integrate into Rust code.

**Rollback Plan:**
If GBNF causes issues:
1. Remove `grammar_path` from profiles
2. System falls back to auto-repair parser (existing behavior)

---

## Related Tasks

- **Task 002:** JSON Repair Intel Unit (model-based repair for edge cases)
- **Task 004:** Schema Validation (complete implementation)
- **Task 006:** Extend Narrative to All Units (uses validated JSON)

---

**Next Action:** Move to `_tasks/active/` and begin Phase 0 implementation.
