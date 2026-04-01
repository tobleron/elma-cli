# Scenario: Ladder 004 - Plan Level (Step-by-Step Refactor Plan)

## Suite
execution_ladder

## File
ladder_004_plan_refactor.md

## Speech Act
INSTRUCTION

## Workflow
PLAN

## Mode
INSPECT

## Route
PLAN

## Expected Formula
plan_reply

## Expected Execution Level
Plan

## User Message
give me a step-by-step plan to refactor orchestration_planning.rs

## Expected Behavior

### Level Selection
- **Level:** Plan
- **Rationale:** User explicitly requests "step-by-step plan", tactical breakdown needed
- **Complexity:** MULTISTEP
- **Risk:** MEDIUM (refactoring can introduce bugs)

### Program Shape
- **Steps:** 2+ (Plan + Reply, or Plan + supporting steps + Reply)
- **Must have Plan step** — Plan level requires explicit planning structure
- **No MasterPlan step** — This is tactical, not strategic

### Expected Program Structure
```json
{
  "objective": "give me a step-by-step plan to refactor orchestration_planning.rs",
  "steps": [
    {
      "type": "plan",
      "id": "p1",
      "goal": "Refactor orchestration_planning.rs for better modularity",
      "purpose": "Create detailed implementation plan",
      "success_condition": "Plan has clear steps with success criteria"
    },
    {
      "type": "reply",
      "id": "r1",
      "instructions": "Present the step-by-step plan to the user",
      "purpose": "Deliver plan to user"
    }
  ]
}
```

### Validation Rules
- ✅ Plan level requires Plan step
- ✅ Plan level allows 2+ steps
- ✅ Plan level doesn't require MasterPlan step
- ✅ Program must have Reply step

## Acceptance Criteria

1. **Level Selection**
   - [ ] `assess_execution_level()` returns `ExecutionLevel::Plan`
   - [ ] `assessment.level == Plan`
   - [ ] `assessment.requires_evidence == true` (needs to understand current code)
   - [ ] `assessment.requires_ordering == true` (steps must be ordered)
   - [ ] `assessment.requires_phases == false` (tactical, not strategic)

2. **Program Generation**
   - [ ] Program has Plan step
   - [ ] Program has 2+ steps
   - [ ] `program_matches_level(&program, ExecutionLevel::Plan)` returns `Ok(())`

3. **Validation**
   - [ ] `program_is_underbuilt(&program, ExecutionLevel::Plan) == false` (has Plan step)
   - [ ] Reject program without Plan step

## Notes

This is a Plan-level request because:
- User explicitly says "step-by-step plan"
- Refactoring requires ordered steps
- Dependencies matter (can't step 3 before step 1)
- Tactical breakdown (not strategic phases)

The ladder should recognize:
- Planning keywords ("step-by-step", "plan")
- MULTISTEP complexity
- Ordering required
- Bounded scope (single file refactor)

## Related Scenarios

- ladder_003_task_evidence_chain.md — Task level (evidence chain)
- ladder_005_masterplan_migration.md — MasterPlan level (strategic)
- ladder_007_underbuild_rejection.md — Rejects missing Plan step
