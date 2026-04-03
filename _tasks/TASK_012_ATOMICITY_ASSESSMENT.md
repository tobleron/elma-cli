# Intel Unit Atomicity Assessment (Task 012)

**Date:** 2026-04-01
**Purpose:** Review all intel units for atomicity and 3B model suitability

---

## Assessment Criteria

### ✅ ATOMIC (Good for 3B models)
- Single clear responsibility
- Prompt < 100 tokens
- One instruction type
- Simple output (boolean, single value, short string)

### ⚠️ MODERATE (Acceptable)
- 1-2 responsibilities
- Prompt 100-200 tokens
- Related instructions
- Structured output (2-3 fields)

### ❌ LOADED (Needs splitting)
- 3+ responsibilities
- Prompt > 200 tokens
- Multiple unrelated instructions
- Complex structured output (4+ fields)

---

## Intel Unit Inventory

### Core Assessment Units

| Unit | Prompt Tokens | Responsibilities | Output Fields | Rating | Action |
|------|--------------|------------------|---------------|--------|--------|
| **complexity_assessor** | ~150 | 1 (assess complexity) | 7 | ⚠️ MODERATE | Split into atomic units |
| **evidence_need_assessor** | ~30 | 1 (needs evidence?) | 2 | ✅ ATOMIC | Keep |
| **action_need_assessor** | ~20 | 1 (needs action?) | 2 | ✅ ATOMIC | Keep |
| **level_assessment** | ~120 | 1 (map to level) | 1 | ✅ ATOMIC | Keep |
| **ordering_needs_assessment** | ~80 | 1 (needs ordering?) | 1 | ✅ ATOMIC | Keep |
| **phases_needs_assessment** | ~80 | 1 (needs phases?) | 1 | ✅ ATOMIC | Keep |
| **revision_needs_assessment** | ~80 | 1 (needs revision?) | 1 | ✅ ATOMIC | Keep |

### Planning Units

| Unit | Prompt Tokens | Responsibilities | Output Fields | Rating | Action |
|------|--------------|------------------|---------------|--------|--------|
| **workflow_planner** | ~120 | 2 (plan + complexity) | 4 | ⚠️ MODERATE | Split |
| **scope_builder** | ~80 | 1 (define scope) | 5 | ⚠️ MODERATE | Simplify |
| **pattern_suggester** | ~30 | 1 (suggest pattern) | 1 | ✅ ATOMIC | Keep |
| **formula_selector** | ~20 | 1 (select formula) | 3 | ✅ ATOMIC | Keep |

### Execution Units

| Unit | Prompt Tokens | Responsibilities | Output Fields | Rating | Action |
|------|--------------|------------------|---------------|--------|--------|
| **selector** | ~100 | 1 (select items) | 2 | ✅ ATOMIC | Keep |
| **evidence_mode** | ~200 | 1 (choose mode) | 2 | ⚠️ MODERATE | Simplify |
| **evidence_compactor** | ~50 | 1 (compact evidence) | 1 | ✅ ATOMIC | Keep |
| **artifact_classifier** | ~50 | 1 (classify artifacts) | 1 | ✅ ATOMIC | Keep |
| **result_presenter** | ~50 | 1 (present results) | 1 | ✅ ATOMIC | Keep |

### Review Units

| Unit | Prompt Tokens | Responsibilities | Output Fields | Rating | Action |
|------|--------------|------------------|---------------|--------|--------|
| **critic** | ~100 | 1 (evaluate program) | 3 | ✅ ATOMIC | Keep |
| **logical_reviewer** | ~100 | 1 (check logic) | 3 | ✅ ATOMIC | Keep |
| **efficiency_reviewer** | ~100 | 1 (check efficiency) | 3 | ✅ ATOMIC | Keep |
| **risk_reviewer** | ~100 | 1 (check risk) | 3 | ✅ ATOMIC | Keep |

### Strategy Units (Task 010)

| Unit | Prompt Tokens | Responsibilities | Output Fields | Rating | Action |
|------|--------------|------------------|---------------|--------|--------|
| **strategy_direct** | ~80 | 1 (direct execution) | N/A | ✅ ATOMIC | Keep |
| **strategy_inspect_first** | ~80 | 1 (inspect first) | N/A | ✅ ATOMIC | Keep |
| **strategy_plan_then_execute** | ~80 | 1 (plan then execute) | N/A | ✅ ATOMIC | Keep |
| **strategy_safe_mode** | ~80 | 1 (safe mode) | N/A | ✅ ATOMIC | Keep |
| **strategy_incremental** | ~80 | 1 (incremental) | N/A | ✅ ATOMIC | Keep |

### Other Units

| Unit | Prompt Tokens | Responsibilities | Output Fields | Rating | Action |
|------|--------------|------------------|---------------|--------|--------|
| **refinement** | ~100 | 1 (refine after drift) | N/A | ✅ ATOMIC | Keep |
| **reflection** | ~100 | 1 (pre-execution reflection) | 4 | ✅ ATOMIC | Keep |
| **outcome_verifier** | ~100 | 1 (verify outcome) | 2 | ✅ ATOMIC | Keep |
| **execution_sufficiency** | ~100 | 1 (check sufficiency) | 3 | ✅ ATOMIC | Keep |

---

## Units Requiring Splitting

### 1. complexity_assessor (SPLIT INTO 4 UNITS)

**Current (LOADED):**
```toml
# complexity_assessor.toml
# Output: complexity, needs_evidence, needs_tools, needs_decision, needs_plan, risk, suggested_pattern
# 7 output fields, complex rules
```

**After (ATOMIC):**
```toml
# complexity_classifier.toml
# Output: complexity only (DIRECT/INVESTIGATE/MULTISTEP/OPEN_ENDED)

# risk_classifier.toml  
# Output: risk only (LOW/MEDIUM/HIGH)

# evidence_needs_classifier.toml
# Output: needs_evidence, needs_tools (2 related fields)

# action_needs_classifier.toml
# Output: needs_decision, needs_plan (2 related fields)
```

**Benefit:** Each unit has 1-2 output fields, simpler prompts.

---

### 2. workflow_planner (SPLIT INTO 2 UNITS)

**Current (MODERATE):**
```toml
# workflow_planner.toml
# Output: objective, complexity, risk, reason
# Mixes planning with complexity assessment
```

**After (ATOMIC):**
```toml
# objective_builder.toml
# Output: objective only

# workflow_metadata.toml
# Output: complexity, risk, reason
```

---

### 3. scope_builder (SIMPLIFY)

**Current (MODERATE):**
```toml
# scope_builder.toml
# Output: focus, include, exclude, query, reason (5 fields)
```

**After (ATOMIC):**
```toml
# scope_builder.toml
# Output: focus, reason only (2 fields)
# include/exclude/query moved to optional fields
```

---

## Splitting Priority

| Priority | Unit | Reason |
|----------|------|--------|
| **HIGH** | complexity_assessor | 7 output fields, complex rules |
| **MEDIUM** | workflow_planner | Mixes responsibilities |
| **LOW** | scope_builder | Can be simplified |

---

## Implementation Plan

### Phase 1: Split complexity_assessor (4 hours)
1. Create 4 new unit configs
2. Update intel_units.rs with 4 new unit structs
3. Update call sites in orchestration_planning.rs
4. Test with 3B model

### Phase 2: Split workflow_planner (2 hours)
1. Create 2 new unit configs
2. Update intel_units.rs
3. Update call sites
4. Test

### Phase 3: Simplify scope_builder (1 hour)
1. Update scope_builder.toml
2. Update intel_units.rs
3. Test

### Phase 4: Verify all units (1 hour)
1. Run all tests
2. Verify prompt lengths < 100 tokens
3. Verify output fields ≤ 3 per unit

---

## Success Metrics

| Metric | Before | Target |
|--------|--------|--------|
| Avg prompt length | ~100 tokens | < 80 tokens |
| Max output fields | 7 | ≤ 3 |
| Units with 1 responsibility | 70% | 95% |
| 3B model success rate | ~60% | ~85% |

---

## Notes

- Most units are already ATOMIC or MODERATE
- Only 3 units need significant changes
- Splitting will improve 3B model reliability
- Plain-text-first output (Task 013) will further improve reliability
