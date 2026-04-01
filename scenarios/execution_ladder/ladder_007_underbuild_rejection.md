# Scenario: Ladder 007 - Underbuild Rejection (No MasterPlan for Strategic Request)

## Suite
execution_ladder

## File
ladder_007_underbuild_rejection.md

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

## Test Type
VALIDATION (Underbuild Rejection)

## Expected Behavior

### Level Selection
- **Level:** MasterPlan
- **Rationale:** Strategic decomposition, multi-phase work, open-ended objective

### Injected Program (Underbuilt)
```json
{
  "objective": "design a phased migration strategy for Elma's planning architecture",
  "steps": [
    {
      "type": "shell",
      "id": "s1",
      "cmd": "ls -la src/",
      "purpose": "List source files",
      "success_condition": "Files listed"
    },
    {
      "type": "reply",
      "id": "r1",
      "instructions": "Here are the files",
      "purpose": "Present file list"
    }
  ]
}
```

### Validation Result
- ❌ `program_matches_level(&program, ExecutionLevel::MasterPlan)` should return `Err(...)`
- ✅ Error message should mention "MasterPlan" or "underbuilt" or "must have"
- ✅ `program_is_underbuilt(&program, ExecutionLevel::MasterPlan) == true`

## Acceptance Criteria

1. **Level Assessment**
   - [ ] `assess_execution_level()` returns `ExecutionLevel::MasterPlan`
   - [ ] `assessment.level == MasterPlan`

2. **Validation**
   - [ ] `program_matches_level(&program, ExecutionLevel::MasterPlan)` returns error
   - [ ] Error message contains "MasterPlan" or "underbuilt" or "must have"
   - [ ] `program_is_underbuilt(&program, ExecutionLevel::MasterPlan) == true`

3. **System Behavior**
   - [ ] System rejects underbuilt program
   - [ ] System can regenerate correct program (MasterPlan-level, with MasterPlan step)
   - [ ] OR system logs warning and proceeds (configurable)

## Notes

This scenario tests that the ladder **rejects underbuilt programs**:
- MasterPlan-level request (strategic decomposition)
- Model provides flat program without MasterPlan structure
- Validation catches the mismatch
- System handles appropriately (reject or warn)

This prevents:
- Strategic requests collapsing to flat task lists
- Missing phased decomposition for complex objectives
- Inadequate planning for high-risk work

## Related Scenarios

- ladder_005_masterplan_migration.md — Correct MasterPlan-level program
- ladder_006_overbuild_rejection.md — Overbuild rejection
- ladder_004_plan_refactor.md — Plan level (also requires planning structure)
