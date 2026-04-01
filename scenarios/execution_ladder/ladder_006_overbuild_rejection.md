# Scenario: Ladder 006 - Overbuild Rejection (Plan for Simple Request)

## Suite
execution_ladder

## File
ladder_006_overbuild_rejection.md

## Speech Act
INSTRUCTION

## Workflow
EXECUTE

## Mode
INSPECT

## Route
SHELL

## Expected Formula
execute_reply

## Expected Execution Level
Action

## User Message
run cargo test

## Test Type
VALIDATION (Overbuild Rejection)

## Expected Behavior

### Level Selection
- **Level:** Action
- **Rationale:** Single primary operation, no decomposition needed

### Injected Program (Overbuilt)
```json
{
  "objective": "run cargo test",
  "steps": [
    {
      "type": "plan",
      "id": "p1",
      "goal": "Plan test execution",
      "purpose": "Create implementation plan",
      "success_condition": "Plan has clear steps"
    },
    {
      "type": "shell",
      "id": "s1",
      "cmd": "cargo test",
      "purpose": "Execute test suite",
      "success_condition": "Tests complete"
    },
    {
      "type": "reply",
      "id": "r1",
      "instructions": "Report test results",
      "purpose": "Present results"
    }
  ]
}
```

### Validation Result
- ❌ `program_matches_level(&program, ExecutionLevel::Action)` should return `Err(...)`
- ✅ Error message should mention "Plan" or "overbuilt"
- ✅ `program_is_overbuilt(&program, ExecutionLevel::Action) == true`

## Acceptance Criteria

1. **Level Assessment**
   - [ ] `assess_execution_level()` returns `ExecutionLevel::Action`
   - [ ] `assessment.level == Action`

2. **Validation**
   - [ ] `program_matches_level(&program, ExecutionLevel::Action)` returns error
   - [ ] Error message contains "Plan" or "overbuilt" or "should not have"
   - [ ] `program_is_overbuilt(&program, ExecutionLevel::Action) == true`

3. **System Behavior**
   - [ ] System rejects overbuilt program
   - [ ] System can regenerate correct program (Action-level, no Plan step)
   - [ ] OR system logs warning and proceeds (configurable)

## Notes

This scenario tests that the ladder **rejects overbuilt programs**:
- Action-level request (simple command)
- Model incorrectly adds Plan structure
- Validation catches the mismatch
- System handles appropriately (reject or warn)

This prevents:
- Unnecessary overhead for simple requests
- Model adding planning structure when not needed
- Wasted tokens and execution time

## Related Scenarios

- ladder_001_action_cargo_test.md — Correct Action-level program
- ladder_007_underbuild_rejection.md — Underbuild rejection
- ladder_002_task_read_summarize.md — Task level (also rejects Plan)
