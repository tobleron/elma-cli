# Scenario: Ladder 005 - MasterPlan Level (Phased Migration Strategy)

## Suite
execution_ladder

## File
ladder_005_masterplan_migration.md

## Speech Act
INSTRUCTION

## Workflow
MASTERPLAN

## Mode
INSPECT

## Route
MASTERPLAN

## Expected Formula
masterplan_reply

## Expected Execution Level
MasterPlan

## User Message
design a phased migration strategy for Elma's planning architecture

## Expected Behavior

### Level Selection
- **Level:** MasterPlan
- **Rationale:** Strategic decomposition, multi-phase work, open-ended objective
- **Complexity:** OPEN_ENDED
- **Risk:** HIGH (architectural changes affect entire system)

### Program Shape
- **Steps:** 2+ (MasterPlan + Reply, or MasterPlan + phases + Reply)
- **Must have MasterPlan step** — MasterPlan level requires strategic decomposition
- **Phases required** — Multi-session work with milestones

### Expected Program Structure
```json
{
  "objective": "design a phased migration strategy for Elma's planning architecture",
  "steps": [
    {
      "type": "masterplan",
      "id": "mp1",
      "goal": "Migrate Elma's planning architecture to execution ladder",
      "purpose": "Create strategic phased decomposition",
      "success_condition": "MasterPlan has clear phases with goals and dependencies"
    },
    {
      "type": "reply",
      "id": "r1",
      "instructions": "Present the phased migration strategy to the user",
      "purpose": "Deliver masterplan to user"
    }
  ]
}
```

### Expected MasterPlan Structure
```json
{
  "goal": "Migrate Elma's planning architecture to execution ladder",
  "phases": [
    {
      "name": "Phase 1: Foundation",
      "objective": "Define ExecutionLevel enum and assessment types",
      "success_criteria": "Types compile, tests pass",
      "dependencies": []
    },
    {
      "name": "Phase 2: Intel Units",
      "objective": "Migrate 4 critical intel units to trait pattern",
      "success_criteria": "Units implement IntelUnit trait, fallbacks work",
      "dependencies": ["Phase 1"]
    },
    {
      "name": "Phase 3: Integration",
      "objective": "Integrate ladder with orchestration",
      "success_criteria": "Ladder assessment used in planning, validation works",
      "dependencies": ["Phase 2"]
    },
    {
      "name": "Phase 4: Testing",
      "objective": "Add scenario tests and verify behavior",
      "success_criteria": "All scenarios pass, metrics collected",
      "dependencies": ["Phase 3"]
    }
  ]
}
```

### Validation Rules
- ✅ MasterPlan level requires MasterPlan step
- ✅ MasterPlan level allows 2+ steps
- ✅ MasterPlan level requires phases (strategic decomposition)
- ✅ Program must have Reply step

## Acceptance Criteria

1. **Level Selection**
   - [ ] `assess_execution_level()` returns `ExecutionLevel::MasterPlan`
   - [ ] `assessment.level == MasterPlan`
   - [ ] `assessment.requires_evidence == true` (needs to understand architecture)
   - [ ] `assessment.requires_ordering == true` (phases must be ordered)
   - [ ] `assessment.requires_phases == true` (strategic decomposition)

2. **Program Generation**
   - [ ] Program has MasterPlan step
   - [ ] Program has 2+ steps
   - [ ] `program_matches_level(&program, ExecutionLevel::MasterPlan)` returns `Ok(())`

3. **Validation**
   - [ ] `program_is_underbuilt(&program, ExecutionLevel::MasterPlan) == false` (has MasterPlan step)
   - [ ] Reject program without MasterPlan step

## Notes

This is a MasterPlan-level request because:
- User says "phased migration strategy"
- Strategic decomposition (not tactical steps)
- Multi-session work (can't complete in one session)
- Open-ended objective (architecture migration)
- High risk (affects entire system)

The ladder should recognize:
- Strategic keywords ("phased", "migration strategy", "architecture")
- OPEN_ENDED complexity
- Phases required
- Multi-session scope

## Related Scenarios

- ladder_004_plan_refactor.md — Plan level (tactical, not strategic)
- ladder_007_underbuild_rejection.md — Rejects missing MasterPlan step
- ladder_006_overbuild_rejection.md — Rejects MasterPlan for simple requests
