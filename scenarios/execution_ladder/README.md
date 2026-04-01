# Execution Ladder Scenarios

This directory contains scenario tests for the Execution Ladder implementation (Task 044).

## Scenarios

| # | File | Level | Purpose |
|---|------|-------|---------|
| 001 | `ladder_001_action_cargo_test.md` | Action | Single operation (cargo test) |
| 002 | `ladder_002_task_read_summarize.md` | Task | Evidence chain (Read → Summarize) |
| 003 | `ladder_003_task_evidence_chain.md` | Task | Evidence chain (Search → Read) |
| 004 | `ladder_004_plan_refactor.md` | Plan | Tactical breakdown (explicit plan) |
| 005 | `ladder_005_masterplan_migration.md` | MasterPlan | Strategic phases |
| 006 | `ladder_006_overbuild_rejection.md` | Action | Rejects Plan for simple request |
| 007 | `ladder_007_underbuild_rejection.md` | MasterPlan | Rejects missing MasterPlan |

## Running Scenarios

```bash
# Run all ladder scenarios
./run_intention_scenarios.sh scenarios/execution_ladder/

# Run specific scenario
./run_intention_scenarios.sh scenarios/execution_ladder/ladder_001_action_cargo_test.md
```

## Expected Results

### Level Selection

| Scenario | Expected Level | Rationale |
|----------|---------------|-----------|
| 001 | Action | Single operation |
| 002 | Task | Evidence chain |
| 003 | Task | Evidence chain (Search→Read) |
| 004 | Plan | Explicit planning request |
| 005 | MasterPlan | Strategic decomposition |

### Validation

| Scenario | Expected Result |
|----------|----------------|
| 001 | ✅ Program matches Action level |
| 002 | ✅ Program matches Task level |
| 003 | ✅ Program matches Task level |
| 004 | ✅ Program matches Plan level |
| 005 | ✅ Program matches MasterPlan level |
| 006 | ❌ Rejects overbuilt program (Plan for Action) |
| 007 | ❌ Rejects underbuilt program (no MasterPlan) |

## Acceptance Criteria

All scenarios must pass:
- [ ] ladder_001_action_cargo_test.md
- [ ] ladder_002_task_read_summarize.md
- [ ] ladder_003_task_evidence_chain.md
- [ ] ladder_004_plan_refactor.md
- [ ] ladder_005_masterplan_migration.md
- [ ] ladder_006_overbuild_rejection.md
- [ ] ladder_007_underbuild_rejection.md

## Metrics to Collect

After running scenarios:

| Metric | Target | Actual |
|--------|--------|--------|
| Level selection accuracy | 100% | _ |
| Overbuild rejection rate | 100% | _ |
| Underbuild rejection rate | 100% | _ |
| False positive rate | <5% | _ |
| Average assessment time | <2s | _ |

## Related Documentation

- `_dev-tasks/TASK_044_PHASE{1,2,3}_COMPLETE.md` — Implementation reports
- `src/execution_ladder.rs` — Ladder implementation
- `src/intel_units.rs` — Migrated intel units
- `src/program_policy.rs` — Level validation

## Troubleshooting

### Scenario Fails: Level Mismatch

If a scenario fails due to level mismatch:

1. Check if the user message contains planning/strategy keywords
2. Check if escalation heuristics triggered (risk, entropy, margin)
3. Check if intel units used fallback (low confidence)

### Scenario Fails: Validation Error

If validation fails unexpectedly:

1. Check step count (Action: 1-3, Task: 2-8, Plan: 2+, MasterPlan: 2+)
2. Check for Plan/MasterPlan steps where not expected
3. Check if Reply step exists (required for all levels)

## See Also

- `_tasks/pending/044_Integrate_Execution_Ladder.md` — Main task
- `_tasks/pending/045_Migrate_Remaining_Intel_Units.md` — Remaining work
- `_dev-tasks/INTEL_UNITS_INVENTORY.md` — Unit catalog
