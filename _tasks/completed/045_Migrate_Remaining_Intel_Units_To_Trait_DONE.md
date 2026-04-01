# Task 045: Migrate Remaining Intel Units to IntelUnit Trait

## Priority
**P2 - MEDIUM** (Architectural consistency, not critical)

## Status
**PENDING** — Depends on Task 034 (trait defined) and Task 044 (4 critical units migrated)

## Context

Task 034 defined the `IntelUnit` trait for standardized intel unit interfaces.
Task 044 migrated 4 critical units for execution ladder:
- `assess_complexity_once()` → `ComplexityAssessmentUnit`
- `assess_evidence_needs_once()` → `EvidenceNeedsUnit`
- `assess_action_needs_once()` → `ActionNeedsUnit`
- `plan_workflow_once()` → `WorkflowPlannerUnit`

This task covers migrating the **remaining 10 intel units** to use the trait.

## Units to Migrate

### Phase 1: Core Orchestration (3 units)

| Function | Unit Name | Profile | Priority |
|----------|-----------|---------|----------|
| `suggest_pattern_once()` | `PatternSuggestionUnit` | `pattern_suggester.toml` | HIGH |
| `build_scope_once()` | `ScopeBuilderUnit` | `scope_builder.toml` | HIGH |
| `select_formula_once()` | `FormulaSelectorUnit` | `formula_selector.toml` | HIGH |

**Rationale:** These are used in the main orchestration flow, benefit from fallback handling.

### Phase 2: Execution Runtime (5 units)

| Function | Unit Name | Profile | Priority |
|----------|-----------|---------|----------|
| `select_items_once()` | `SelectorUnit` | `selector.toml` | MEDIUM |
| `decide_evidence_mode_once()` | `EvidenceModeUnit` | `evidence_mode.toml` | MEDIUM |
| `compact_evidence_once()` | `EvidenceCompactorUnit` | ⚠️ Need profile | MEDIUM |
| `classify_artifacts_once()` | `ArtifactClassifierUnit` | `artifact_classifier.toml` | MEDIUM |
| `present_result_once()` | `ResultPresenterUnit` | `result_presenter.toml` | MEDIUM |

**Rationale:** Used during execution, fallback handling improves reliability.

**Note:** `compact_evidence_once()` needs a new profile created.

### Phase 3: Helper Units (2 units)

| Function | Unit Name | Profile | Priority |
|----------|-----------|---------|----------|
| `generate_status_message_once()` | `StatusMessageUnit` | `status_message_generator.toml` | LOW |
| `repair_command_once()` | `CommandRepairUnit` | `command_repair.toml` | LOW |

**Rationale:** Nice to have for consistency, but not critical.

## Implementation Steps

### For Each Unit

1. **Create unit struct** in `src/intel_units/` (or keep in `src/intel.rs`):
   ```rust
   pub struct PatternSuggestionUnit {
       profile: Profile,
   }
   
   impl IntelUnit for PatternSuggestionUnit {
       fn name(&self) -> &'static str { "pattern_suggestion" }
       fn profile(&self) -> &Profile { &self.profile }
       
       async fn execute(&self, context: &IntelContext) -> Result<IntelOutput> {
           // Migrate existing logic from function
       }
       
       fn post_flight(&self, output: &IntelOutput) -> Result<()> {
           // Add output validation
       }
       
       fn fallback(&self, context: &IntelContext, error: &str) -> Result<IntelOutput> {
           // Add safe default
       }
   }
   ```

2. **Keep old function as compatibility wrapper**:
   ```rust
   // Old function still works, calls new unit internally
   pub async fn suggest_pattern_once(
       client: &reqwest::Client,
       chat_url: &Url,
       cfg: &Profile,
       // ... other params
   ) -> Result<String> {
       let unit = PatternSuggestionUnit::new(cfg.clone());
       let context = IntelContext::new(...);
       let output = unit.execute_with_fallback(&context).await?;
       Ok(output.get_str("suggested_pattern").unwrap_or("reply_only").to_string())
   }
   ```

3. **Update call sites gradually** (or leave as-is with wrapper):
   ```rust
   // Old way (still works):
   let pattern = suggest_pattern_once(...).await?;
   
   // New way (optional):
   let unit = PatternSuggestionUnit::new(profile);
   let output = unit.execute_with_fallback(&context).await?;
   ```

4. **Add tests** for each unit:
   ```rust
   #[test]
   fn test_pattern_suggestion_unit() {
       // Test trait methods
   }
   ```

## Acceptance Criteria

### Phase 1 (Core Orchestration)
- [ ] `PatternSuggestionUnit` implemented with trait
- [ ] `ScopeBuilderUnit` implemented with trait
- [ ] `FormulaSelectorUnit` implemented with trait
- [ ] Old functions work as compatibility wrappers
- [ ] Tests pass for all 3 units

### Phase 2 (Execution Runtime)
- [ ] `SelectorUnit` implemented with trait
- [ ] `EvidenceModeUnit` implemented with trait
- [ ] `EvidenceCompactorUnit` implemented with trait
- [ ] `ArtifactClassifierUnit` implemented with trait
- [ ] `ResultPresenterUnit` implemented with trait
- [ ] Profile created for `compact_evidence_once()`
- [ ] Tests pass for all 5 units

### Phase 3 (Helpers)
- [ ] `StatusMessageUnit` implemented with trait
- [ ] `CommandRepairUnit` implemented with trait
- [ ] Tests pass for both units

### Overall
- [ ] All 14 intel units use `IntelUnit` trait
- [ ] Zero warnings in build
- [ ] All tests pass
- [ ] No breaking changes to existing call sites
- [ ] Fallback handling works for all units
- [ ] Documentation updated

## Files to Create

| File | Purpose |
|------|---------|
| `src/intel_units.rs` (or expand `src/intel_trait.rs`) | Unit implementations |
| `config/defaults/evidence_compactor.toml` | Profile for compact_evidence |

## Files to Modify

| File | Change |
|------|--------|
| `src/intel.rs` | Add compatibility wrappers |
| `src/main.rs` | Export new units |

## Estimated Effort

| Phase | Units | Hours |
|-------|-------|-------|
| Phase 1 (Core) | 3 | 3-4 |
| Phase 2 (Execution) | 5 | 5-7 |
| Phase 3 (Helpers) | 2 | 2-3 |
| **Total** | **10** | **10-14 hours** |

## Benefits

| Benefit | Impact |
|---------|--------|
| **Consistency** | All units use same interface |
| **Fallback handling** | Automatic safe defaults on failure |
| **Input validation** | Pre-flight checks prevent bad calls |
| **Output validation** | Post-flight checks catch bad responses |
| **Confidence tracking** | Know when model was uncertain |
| **Testability** | Easy to mock and test in isolation |
| **Composability** | Units chain easily with standardized I/O |

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| **Breaking changes** | Low | High | Keep old functions as wrappers |
| **Profile drift** | Low | Medium | Copy prompts exactly from existing profiles |
| **Test coverage gaps** | Medium | Low | Write tests for each unit |
| **Call site updates** | Medium | Medium | Use wrappers, update gradually |

## Dependencies

- ✅ **Task 034** (Intel Unit Trait) — COMPLETE
- ⏳ **Task 044** (4 Critical Units) — Must complete first
- ⏳ **Task 009** (JSON Fallback) — Infrastructure exists

## Relationship to Other Tasks

| Task | Relationship |
|------|--------------|
| Task 034 | Uses trait defined in 034 |
| Task 044 | Continues migration started in 044 |
| Task 001 | Reflection can use migrated units |
| Task 013 | Classification features used by units |

## Verification

```bash
# Build verification
cargo build 2>&1 | grep -E "warning|error"

# Test all intel units
cargo test intel

# Run scenario probes
./run_intention_scenarios.sh
```

## Success Metrics

| Metric | Target |
|--------|--------|
| Units migrated | 10/10 |
| Tests passing | 100% |
| Warnings | 0 |
| Breaking changes | 0 |
| Fallback coverage | 100% |

## Notes

- **Don't rush this task** — migrate units one at a time
- **Test thoroughly** — each unit needs tests
- **Keep wrappers** — old functions should continue working
- **No prompt changes** — copy system prompts exactly

## See Also

- `_dev-tasks/INTEL_UNITS_INVENTORY.md` — Complete unit catalog
- `_dev-tasks/TASK_034_COMPLETION_REPORT.md` — Trait definition
- `_tasks/pending/044_Integrate_Execution_Ladder.md` — First 4 units
