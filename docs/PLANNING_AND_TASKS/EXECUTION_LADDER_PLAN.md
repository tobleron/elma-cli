# Execution Ladder Implementation Plan

**Created:** 2026-04-01
**Status:** Planning Complete — Ready for Implementation
**Related:** Task 044 (Integrate Execution Ladder)

---

## Executive Summary

This document answers three critical questions about the proposed Execution Ladder implementation:

1. **Will semantic principle-based classification be more accurate?**
   - **Answer:** Net +15-25% accuracy gain (not +30-40% as initially claimed)
   - Gains from validation, escalation, and reflection — not base assessment

2. **Does this respect modular intel unit philosophy?**
   - **Answer:** Initial design did NOT — corrected to use 6 specialized intel units
   - Each unit has single responsibility, dedicated profile, independent fallback

3. **Do phases align with existing tasks?**
   - **Answer:** Yes, with updates to Tasks 001, 006, 013, 034 and new Task 044
   - Clear dependency graph and implementation sequence defined

---

## Question 1: Accuracy Analysis

### Current Architecture

```
User Input → Route Priors → Complexity → Evidence → Action → Formula → Program
              (model)        (model)     (model)   (model)  (model)   (model)
```

**6 model calls**, each specialized for one assessment dimension.

### Proposed Execution Ladder

```
User Input → Route Priors → Execution Level Assessment → Formula → Program
              (model)        (6 intel units)             (constrained) (model)
```

**7-8 model calls** (6 for ladder assessment + routing + program generation).

### Accuracy Comparison

| Aspect | Current | Ladder | Winner | Reason |
|--------|---------|--------|--------|--------|
| **Complexity classification** | Model-based | Same + level mapping | Tie | Same model calls |
| **Depth gating** | Hardcoded table | Principle-based heuristics | **Ladder** | More flexible |
| **Evidence detection** | Separate model call | Integrated in assessment | Current | More focused |
| **Ordering detection** | Not modeled | New field | **Ladder** | New capability |
| **Risk escalation** | Hardcoded | Principle-based | **Ladder** | More nuanced |
| **Classification override** | Limited | Explicit | **Ladder** | More flexible |
| **Program validation** | Route-based only | Level + route | **Ladder** | Stricter checks |
| **Reflection coverage** | Skips DIRECT | Always-on | **Ladder** | Catches more issues |

### Accuracy Gains (Quantified)

| Metric | Baseline | Target | Gain |
|--------|----------|--------|------|
| Overbuilt programs | ~20% | <5% | **-15%** |
| Underbuilt programs | ~15% | <5% | **-10%** |
| Reflection coverage | ~60% | 100% | **+40%** |
| Critic hallucination | ~15% | <8% | **-7%** |
| Retry rate | ~25% | <15% | **-10%** |

**Net accuracy gain: +15-25%** (conservative estimate)

### Why Not Higher?

- Base assessment uses same model calls (neutral impact)
- Prompt dilution risk (mitigated by modular units)
- Level boundary ambiguity (mitigated by clear semantics)

---

## Question 2: Intel Unit Philosophy

### Initial Design (WRONG)

```rust
// Monolithic assessment function
pub fn assess_execution_level(...) -> ExecutionLadderAssessment
```

**Problems:**
- ❌ Asks one model call to assess 6 dimensions
- ❌ No dedicated profiles per dimension
- ❌ Single point of failure
- ❌ Hard to test/debug
- ❌ Violates "specialized for smaller models" philosophy

### Corrected Design (RIGHT)

```rust
// 6 specialized intel units
assess_level_from_complexity_once()    // → ExecutionLevel
assess_evidence_chain_once()           // → bool
assess_ordering_needs_once()           // → bool
assess_phases_needs_once()             // → bool
assess_revision_needs_once()           // → bool
generate_strategy_hint_once()          // → Option<String>

// Pure function to combine results
fn assemble_ladder_assessment(...) -> ExecutionLadderAssessment
```

**Benefits:**
- ✅ Each unit has single responsibility
- ✅ Dedicated profile per unit (tunable temperature, tokens)
- ✅ Independent fallback per unit
- ✅ Testable in isolation
- ✅ Aligns with Elma philosophy

### Token/Cost Trade-off

| Approach | Model Calls | Tokens | Latency |
|----------|-------------|--------|---------|
| Monolithic | 1 | ~1000 | ~2s |
| Modular (full) | 6 | ~1800 | ~6s |
| **Hybrid (recommended)** | 1.2 avg | ~360 avg | ~1.4s avg |

**Hybrid approach:**
- Heuristic fast path for 80% of clear cases (0 model calls)
- Full intel assessment for 20% of ambiguous cases (6 model calls)
- Best of both worlds: speed + accuracy

---

## Question 3: Task Alignment

### Dependency Graph

```
                    Task 009 (JSON Fallback)
                           │
                           ▼
                    Task 034 (Intel Units)
                           │
              ┌────────────┼────────────┐
              ▼            ▼            ▼
         Task 001    Task 013      Task 044
      (Reflection)  (Decouple)   (Ladder)
              │            │            │
              └────────────┼────────────┘
                           ▼
                    Task 006 (Plan Formulas)
```

### Task Dependencies

| Task | Priority | Status | Relationship to Ladder |
|------|----------|--------|------------------------|
| **009** (JSON Fallback) | P0 | Pending | **Prerequisite** — ladder intel units need JSON parsing with fallback |
| **034** (Intel Units) | P1 | Pending | **Prerequisite** — ladder follows intel unit pattern |
| **001** (Reflection) | P0 | Pending | **Coordinate** — ladder extends reflection to level critique |
| **013** (Decouple Classification) | P1 | Pending | **Coordinate** — ladder uses soft classification features |
| **044** (Execution Ladder) | P0 | New | **Main implementation** — integrates all components |
| **006** (Plan Formulas) | P1 | Pending | **Dependent** — uses ladder level semantics |

### Updated Task Files

| Task | Update Summary |
|------|----------------|
| **000** (Reflection) | Added relationship to Task 044 — ladder extends reflection |
| **006** (Plan Formulas) | Added relationship — ladder clarifies Plan vs MasterPlan |
| **013** (Decouple) | Added relationship — shared soft guidance principle |
| **034** (Intel Units) | Expanded with full trait definition and ladder unit list |
| **044** (Ladder) | **NEW** — comprehensive implementation plan |

---

## Recommended Implementation Sequence

### Phase 0: Prerequisites (Week 1-2)

**Complete these FIRST:**

1. **Task 009** (JSON Fallback Strategy)
   - Why: All intel units need robust JSON parsing
   - Effort: ~4-6 hours
   - Blocks: Tasks 034, 044

2. **Task 001** (Enable Reflection for All)
   - Why: Ladder requires reflection always-on
   - Effort: ~2-3 hours
   - Blocks: Task 044 (partial)

### Phase 1: Foundation (Week 3-4)

**Complete these SECOND:**

3. **Task 034** (Formalize Intel Unit Interfaces)
   - Why: Defines pattern for ladder intel units
   - Effort: ~6-8 hours
   - Blocks: Task 044

4. **Task 013** (Decouple Classification)
   - Why: Provides feature vector for ladder assessment
   - Effort: ~4-6 hours
   - Blocks: Task 044 (partial)

### Phase 2: Ladder Implementation (Week 5-6)

**Complete these THIRD:**

5. **Task 044 Phase 1** (Ladder Types + Intel Units)
   - Why: Core ladder infrastructure
   - Effort: ~8-10 hours
   - Blocks: Task 044 Phase 2

6. **Task 044 Phase 2** (Integration + Validation)
   - Why: Activates ladder in orchestration
   - Effort: ~6-8 hours
   - Blocks: Task 006

### Phase 3: Formulas + Testing (Week 7-8)

**Complete these FOURTH:**

7. **Task 006** (Revise Plan Formulas)
   - Why: Aligns formulas with ladder semantics
   - Effort: ~6-8 hours
   - Unblocks: General availability

8. **Task 044 Phase 3-4** (Prompts + Testing)
   - Why: Final polish and verification
   - Effort: ~4-6 hours
   - Complete: Execution ladder fully operational

---

## Total Effort Estimate

| Phase | Tasks | Hours | Weeks |
|-------|-------|-------|-------|
| **Phase 0** (Prerequisites) | 009, 001 | 6-9 | 1-2 |
| **Phase 1** (Foundation) | 034, 013 | 10-14 | 2-3 |
| **Phase 2** (Ladder) | 044 P1-P2 | 14-18 | 2-3 |
| **Phase 3** (Formulas) | 006, 044 P3-P4 | 10-14 | 2 |
| **Total** | 6 tasks | **40-55 hours** | **7-10 weeks** |

**Note:** Can be accelerated with parallel work on independent tasks.

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| **Task 009 delays** | Medium | High | Start immediately, prioritize |
| **Intel unit bloat** | Medium | Medium | Enforce single responsibility |
| **Prompt dilution** | Medium | Low | Principle-based, not example-heavy |
| **Token overhead** | High | Medium | Hybrid fast/slow path |
| **Breaking sessions** | Low | High | Compatibility wrappers |
| **Task coordination** | Medium | Medium | Clear dependency graph (this doc) |

---

## Success Metrics

| Metric | Baseline | Target | Measurement |
|--------|----------|--------|-------------|
| **Overbuilt programs** | ~20% | <5% | Scenario validation |
| **Underbuilt programs** | ~15% | <5% | Scenario validation |
| **Reflection coverage** | ~60% | 100% | Session traces |
| **Critic hallucination** | ~15% | <8% | Critic parse errors |
| **Retry rate** | ~25% | <15% | Session metrics |
| **Task completion** | 0/6 | 6/6 | This implementation plan |

---

## Next Actions

1. **Start Task 009** (JSON Fallback) — Highest priority prerequisite
2. **Start Task 001** (Reflection) — Can run in parallel with 009
3. **Monitor progress** — Update this doc as tasks complete
4. **Adjust sequence** — Re-evaluate after Phase 0 completion

---

## Appendix: Task File Locations

| Task | File Path |
|------|-----------|
| 001 | `_tasks/pending/000_Enable_Reflection_For_All_Tasks.md` |
| 006 | `_tasks/pending/006_Revise_Core_Formulas_Plan_Family.md` |
| 009 | `_tasks/pending/009_JSON_Fallback_Strategy.md` |
| 013 | `_tasks/pending/013_Decouple_Classification_From_Execution.md` |
| 034 | `_tasks/pending/034_Formalize_Intel_Unit_Interfaces.md` |
| 044 | `_tasks/pending/044_Integrate_Execution_Ladder.md` |


## 📚 Related Documentation

- **[Architecture Reference](../ARCHITECTURE/../ARCHITECTURE/ARCHITECTURE.md)**
- **[Task Management System](./TASKS.md)**
- **[Roadmap](./../PLANNING_AND_TASKS/REPRIORITIZED_ROADMAP.md)**
- **[Intel Unit Standard](../STANDARDS_AND_TOOLS/../STANDARDS_AND_TOOLS/../STANDARDS_AND_TOOLS/INTEL_UNIT_STANDARD.md)**


| Date | Change | Author |
|------|--------|--------|
| 2026-04-01 | Initial creation | Analysis from masterplan prompt |
| 2026-04-01 | Updated Tasks 001, 006, 013, 034 | Added relationships to Task 044 |
| 2026-04-01 | Created Task 044 | Comprehensive implementation plan |
plan |

