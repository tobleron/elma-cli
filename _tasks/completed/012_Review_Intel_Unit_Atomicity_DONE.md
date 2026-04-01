# Task 012: Review Intel Unit Atomicity for Small Models

## Priority
**P1 - HIGH** (Critical for 3B model compatibility)

## Context
Elma's philosophy states it is "specialized for smaller llm models with constrained hardware resources."

However, intel units may have:
- Complex multi-part prompts
- Multiple responsibilities per unit
- Expectations beyond 3B model capabilities

## Problem
Small models (3B parameters) struggle with:
- Prompts longer than 50-100 tokens
- Multiple instructions in single prompt
- Complex reasoning chains
- Direct JSON generation without scaffolding

If intel units are not atomic, small models will:
- Fail to follow all instructions
- Miss key requirements
- Produce inconsistent output
- Have high fallback rates

## Objective
Review all intel units for atomicity and small-model suitability.

## Technical Tasks

### 1. Inventory All Intel Units

Create a spreadsheet documenting:
- Unit name
- Prompt length (tokens)
- Number of distinct instructions
- Output complexity (simple value vs. structured object)
- Current fallback rate (if available)

### 2. Atomicity Assessment

For each unit, check:

**✅ ATOMIC (Good for 3B):**
- Single clear responsibility
- Prompt < 100 tokens
- One instruction type
- Simple output (boolean, single value, short string)

**❌ LOADED (Needs splitting):**
- Multiple responsibilities
- Prompt > 200 tokens
- Multiple instruction types ("do X, then Y, also check Z")
- Complex structured output

### 3. Identify Units Needing Split

Examples of potential splits:

**Before (loaded):**
```toml
# complexity_assessor.toml
system_prompt = """
Assess task complexity and risk level.
Consider:
- User intent
- Workspace context
- Classification priors
- Potential obstacles
- Required evidence

Return JSON: {
  "complexity": "DIRECT" | "INVESTIGATE" | "MULTISTEP" | "OPEN_ENDED",
  "risk": "LOW" | "MEDIUM" | "HIGH",
  "needs_evidence": bool,
  "needs_tools": bool,
  "needs_decision": bool,
  "needs_plan": bool,
  "suggested_pattern": "..."
}
"""
```

**After (atomic):**
```toml
# complexity_assessor.toml
system_prompt = """
Assess task complexity only.
Return ONE word: DIRECT, INVESTIGATE, MULTISTEP, or OPEN_ENDED.
"""

# risk_assessor.toml
system_prompt = """
Assess task risk level only.
Return ONE word: LOW, MEDIUM, or HIGH.
"""

# evidence_needs_assessor.toml
system_prompt = """
Does this task need workspace evidence?
Return true or false.
"""
```

### 4. Create Splitting Plan

For each loaded unit:
- Identify natural split points
- Create new unit definitions
- Update call sites
- Test with 3B model

### 5. Test with 3B Model

For each unit (before and after splitting):
- Run 10 test cases
- Measure success rate
- Measure fallback rate
- Compare output quality

## Acceptance Criteria
- [ ] All intel units documented with atomicity assessment
- [ ] Loaded units identified and split plan created
- [ ] At least 5 loaded units split into atomic units
- [ ] 3B model success rate improved by 20%+ on split units
- [ ] Fallback rate reduced by 30%+ on split units

## Files to Modify
- `src/intel_units.rs` - Split loaded units
- `config/defaults/*.toml` - New atomic unit prompts
- `src/orchestration_planning.rs` - Update call sites

## Estimated Effort
8-12 hours

## Philosophy Alignment
- ✅ "Specialized for smaller llm models"
- ✅ "Maximize intelligence per token"
- ✅ "Modular intel units"
- ✅ "Accuracy and reliability over speed"

## Success Metrics

| Metric | Before | Target |
|--------|--------|--------|
| Avg prompt length | ~150 tokens | <100 tokens |
| Instructions per unit | ~3 | 1 |
| 3B model success rate | ~60% | ~85% |
| Fallback rate | ~25% | <10% |
