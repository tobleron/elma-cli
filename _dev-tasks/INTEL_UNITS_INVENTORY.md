# Intel Units Inventory

**Date:** 2026-04-01
**Purpose:** Count and catalog all intel units for migration planning

---

## Summary

**Total Intel Functions:** **14 functions** in `src/intel.rs`

**Already Have Profiles:** **13 profiles** (some functions share profiles)

**Need New Profiles:** **1 profile** (`status_message_generator` - may not need one)

---

## Complete List

### Core Assessment Units (6) - CRITICAL FOR TASK 044

| # | Function | Profile | Status | Priority |
|---|----------|---------|--------|----------|
| 1 | `assess_complexity_once()` | `complexity_assessor.toml` | ✅ Exists | 🔴 **MIGRATE FIRST** |
| 2 | `assess_evidence_needs_once()` | `evidence_need_assessor.toml` | ✅ Exists | 🔴 **MIGRATE FIRST** |
| 3 | `assess_action_needs_once()` | `action_need_assessor.toml` | ✅ Exists | 🔴 **MIGRATE FIRST** |
| 4 | `suggest_pattern_once()` | `pattern_suggester.toml` | ✅ Exists | 🟡 Migrate second |
| 5 | `build_scope_once()` | `scope_builder.toml` | ✅ Exists | 🟡 Migrate second |
| 6 | `select_formula_once()` | `formula_selector.toml` | ✅ Exists | 🟡 Migrate second |

**Note:** Functions 1-3 are needed for Task 044 ladder assessment.

---

### Planning Units (1) - CRITICAL FOR ORCHESTRATION

| # | Function | Profile | Status | Priority |
|---|----------|---------|--------|----------|
| 7 | `plan_workflow_once()` | `workflow_planner.toml` + `workflow_complexity_planner.toml` + `workflow_reason_planner.toml` | ✅ Exists (3 profiles) | 🔴 **MIGRATE FIRST** |

**Note:** Uses 3 profiles (scope, complexity, reason).

---

### Execution Units (5) - NEEDED FOR RUNTIME

| # | Function | Profile | Status | Priority |
|---|----------|---------|--------|----------|
| 8 | `select_items_once()` | `selector.toml` | ✅ Exists | 🟢 Migrate later |
| 9 | `decide_evidence_mode_once()` | `evidence_mode.toml` | ✅ Exists | 🟢 Migrate later |
| 10 | `compact_evidence_once()` | (no profile found) | ⚠️ Need profile | 🟢 Migrate later |
| 11 | `classify_artifacts_once()` | `artifact_classifier.toml` | ✅ Exists | 🟢 Migrate later |
| 12 | `present_result_once()` | `result_presenter.toml` | ✅ Exists | 🟢 Migrate later |

---

### Helper Units (2) - LOWER PRIORITY

| # | Function | Profile | Status | Priority |
|---|----------|---------|--------|----------|
| 13 | `generate_status_message_once()` | `status_message_generator.toml` | ✅ Exists | 🟢 Migrate later (or never) |
| 14 | `repair_command_once()` | `command_repair.toml` | ✅ Exists | 🟢 Migrate later |

---

## Migration Priority

### Phase 1: Task 044 Prerequisites (CRITICAL)

**Must migrate first** - Task 044 depends on these:

1. `assess_complexity_once()` → `ComplexityAssessmentUnit`
2. `assess_evidence_needs_once()` → `EvidenceNeedsUnit`
3. `assess_action_needs_once()` → `ActionNeedsUnit`
4. `plan_workflow_once()` → `WorkflowPlannerUnit`

**Estimated effort:** 4-6 hours

---

### Phase 2: Core Orchestration (HIGH)

**Should migrate second** - Used in main orchestration flow:

5. `suggest_pattern_once()` → `PatternSuggestionUnit`
6. `build_scope_once()` → `ScopeBuilderUnit`
7. `select_formula_once()` → `FormulaSelectorUnit`

**Estimated effort:** 4-6 hours

---

### Phase 3: Execution Runtime (MEDIUM)

**Can migrate later** - Used during execution:

8. `select_items_once()` → `SelectorUnit`
9. `decide_evidence_mode_once()` → `EvidenceModeUnit`
10. `compact_evidence_once()` → `EvidenceCompactorUnit` (need profile)
11. `classify_artifacts_once()` → `ArtifactClassifierUnit`
12. `present_result_once()` → `ResultPresenterUnit`

**Estimated effort:** 6-8 hours

---

### Phase 4: Helpers (LOW)

**Optional** - Nice to have, not critical:

13. `generate_status_message_once()` → `StatusMessageUnit`
14. `repair_command_once()` → `CommandRepairUnit`

**Estimated effort:** 2-4 hours

---

## Total Effort Estimate

| Phase | Units | Hours | Cumulative |
|-------|-------|-------|------------|
| Phase 1 (Task 044) | 4 | 4-6 | 4-6 |
| Phase 2 (Orchestration) | 3 | 4-6 | 8-12 |
| Phase 3 (Execution) | 5 | 6-8 | 14-20 |
| Phase 4 (Helpers) | 2 | 2-4 | 16-24 |

**Total:** 16-24 hours for full migration

---

## Profiles Status

### ✅ Existing Profiles (13)

```
complexity_assessor.toml
evidence_need_assessor.toml
action_need_assessor.toml
pattern_suggester.toml
scope_builder.toml
formula_selector.toml
workflow_planner.toml
workflow_complexity_planner.toml
workflow_reason_planner.toml
selector.toml
evidence_mode.toml
artifact_classifier.toml
result_presenter.toml
status_message_generator.toml
command_repair.toml
```

### ⚠️ Missing Profiles (1)

```
compact_evidence_assessment.toml  (for compact_evidence_once)
```

---

## Recommendation

### For Task 044 (Immediate Need)

**Migrate ONLY these 4 units:**
1. `assess_complexity_once()`
2. `assess_evidence_needs_once()`
3. `assess_action_needs_once()`
4. `plan_workflow_once()`

**Why:**
- Task 044 needs these for ladder assessment
- Existing functions work but lack fallback handling
- Migration gives immediate reliability benefit

**Effort:** 4-6 hours

---

### For Full Migration (Optional)

**Migrate all 14 units** for:
- Consistency (all units use trait)
- Better error handling (automatic fallbacks)
- Easier testing (mock trait)
- Confidence tracking

**Effort:** 16-24 hours total

---

## Decision Point

**Option A: Minimal (Task 044 only)**
- Migrate 4 units
- 4-6 hours
- Task 044 can proceed

**Option B: Core (Task 044 + orchestration)**
- Migrate 7 units
- 8-12 hours
- Better orchestration reliability

**Option C: Full (All units)**
- Migrate 14 units
- 16-24 hours
- Complete architectural consistency

---

## My Recommendation

**Start with Option A (4 units for Task 044)**, then:
- If reliability improves noticeably → Continue with Option B
- If no noticeable improvement → Stop, use hybrid approach

**Don't migrate units that work fine unless there's a clear benefit.**
