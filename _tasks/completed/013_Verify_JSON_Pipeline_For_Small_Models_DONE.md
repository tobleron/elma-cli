# Task 013: Verify JSON Pipeline for Small Models

## Priority
**P1 - HIGH** (Critical for 3B model reliability)

## Context
Elma's philosophy states it is "specialized for smaller llm models with constrained hardware resources."

Small models (3B parameters) struggle with:
- Direct JSON generation
- Complex schema adherence
- Escaping special characters correctly
- Maintaining valid JSON structure under pressure

## Current Architecture Question

**Do intel units:**
1. Generate plain text first, then JSON converter extracts JSON? ✅ (Ideal for 3B)
2. Generate JSON directly from model? ❌ (Hard for 3B)

## Investigation Needed

### 1. Map JSON Generation Flow

For each intel unit that outputs JSON:

```
Unit → ??? → JSON Output
       ↓
   Is there a converter?
```

**Expected (good for 3B):**
```
Model → Plain text response → JSON extractor → Structured JSON
        (model writes naturally)  (deterministic)  (validated)
```

**Current (potentially problematic):**
```
Model → JSON output (direct)
        (3B model struggles with JSON syntax)
```

### 2. Check Existing JSON Infrastructure

Elma has:
- `src/json_error_handler.rs` - Fallback handling
- `src/routing_parse.rs` - JSON extraction (`extract_first_json_object`)
- `chat_json_with_repair()` - JSON repair pipeline
- `chat_json_with_repair_text()` - Text extraction + repair

**Question:** Are these used for intel unit outputs, or only for orchestrator/critic outputs?

### 3. Verify Intel Unit Output Flow

Check each intel unit in `src/intel_units.rs`:

```rust
// Example: ComplexityAssessmentUnit
async fn execute(&self, context: &IntelContext) -> Result<IntelOutput> {
    let result: serde_json::Value = chat_json_with_repair_timeout(...).await?;
    // ↓ This calls chat_once() which extracts text, then parses JSON
    // Is there a plain-text-first step?
}
```

**Trace the flow:**
1. `chat_json_with_repair_timeout()` → ?
2. `chat_once()` → ?
3. `extract_response_text()` → ?
4. `parse_json_loose()` → ?

### 4. Identify Gaps

If intel units expect direct JSON from 3B models:

**Problem:**
- 3B models make JSON syntax errors
- Missing quotes, trailing commas, unescaped strings
- High fallback rate due to parse failures

**Solution:**
- Model outputs plain text: "The complexity is DIRECT with LOW risk"
- Deterministic extractor parses: `{complexity: "DIRECT", risk: "LOW"}`
- Much higher success rate for 3B models

### 5. Create Plain-Text-First Pipeline

For each intel unit:

**Before (direct JSON):**
```toml
system_prompt = """
Return ONLY valid JSON:
{
  "complexity": "DIRECT",
  "risk": "LOW"
}
"""
```

**After (plain text first):**
```toml
system_prompt = """
Answer in plain text:
- What is the complexity? (DIRECT/INVESTIGATE/MULTISTEP/OPEN_ENDED)
- What is the risk level? (LOW/MEDIUM/HIGH)

Example: "The complexity is DIRECT and the risk is LOW."
"""
```

**Add extractor:**
```rust
fn extract_complexity_from_text(text: &str) -> Result<ComplexityAssessment> {
    // Deterministic extraction from plain text
    // Much more reliable than JSON parsing for 3B models
}
```

## Acceptance Criteria
- [ ] JSON generation flow documented for all intel units
- [ ] Plain-text-first pipeline identified or created
- [ ] At least 5 intel units converted to plain-text-first
- [ ] 3B model JSON parse failure rate <5%
- [ ] Fallback rate reduced by 40%+ for JSON-producing units

## Files to Modify
- `src/intel_units.rs` - Update units to use plain-text prompts
- `src/intel_trait.rs` - Add plain-text extraction helpers
- `config/defaults/*.toml` - Update prompts for plain-text output

## Estimated Effort
6-10 hours

## Philosophy Alignment
- ✅ "Specialized for smaller llm models"
- ✅ "Maximize intelligence per token"
- ✅ "Accuracy and reliability over speed"
- ✅ "Aggressive compression techniques"

## Success Metrics

| Metric | Before | Target |
|--------|--------|--------|
| JSON parse failures | ~15% | <5% |
| Fallback rate (JSON units) | ~30% | <10% |
| 3B model success rate | ~60% | ~90% |
| Avg repair iterations | ~2.5 | <1.0 |

## Example: Plain-Text-First Flow

**Task:** Assess complexity

**Old flow (direct JSON):**
```
Model prompt: "Return JSON: {complexity, risk}"
Model output: {"complexity": "DIRECT", "risk": "LOW"}  ← 3B model often breaks JSON
Parse: Fail → Repair → Fail → Fallback
```

**New flow (plain text first):**
```
Model prompt: "What is the complexity and risk?"
Model output: "The complexity is DIRECT and risk is LOW."  ← Natural language
Extractor: Regex/deterministic parse → {complexity: "DIRECT", risk: "LOW"}
Parse: Success (no JSON to break)
```

## Relationship to Task 012

**Task 012 (Atomicity)** + **Task 013 (JSON Pipeline)** = **Small Model Optimization**

- Task 012: Split complex prompts into atomic units
- Task 013: Make each atomic unit output plain text

**Together:** 3B models can handle each unit reliably
