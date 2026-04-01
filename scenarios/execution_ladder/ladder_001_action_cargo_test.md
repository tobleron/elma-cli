# Scenario: Ladder 001 - Action Level (Run Cargo Test)

## Suite
execution_ladder

## File
ladder_001_action_cargo_test.md

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

## Expected Behavior

### Level Selection
- **Level:** Action
- **Rationale:** Single primary operation, no decomposition needed
- **Complexity:** DIRECT
- **Risk:** LOW

### Program Shape
- **Steps:** 1-2 (Shell + Reply)
- **No Plan step** — Action level doesn't need planning structure
- **No MasterPlan step** — Action level doesn't need strategic decomposition

### Expected Program Structure
```json
{
  "objective": "run cargo test",
  "steps": [
    {
      "type": "shell",
      "id": "s1",
      "cmd": "cargo test",
      "purpose": "Execute test suite",
      "success_condition": "Tests complete with exit code 0"
    },
    {
      "type": "reply",
      "id": "r1",
      "instructions": "Report test results",
      "purpose": "Present test output to user"
    }
  ]
}
```

### Validation Rules
- ✅ Action level allows 1-3 steps
- ✅ Action level rejects Plan step
- ✅ Action level rejects MasterPlan step
- ✅ Program must have Reply step

## Acceptance Criteria

1. **Level Assessment**
   - [ ] `assess_execution_level()` returns `ExecutionLevel::Action`
   - [ ] `assessment.level == Action`
   - [ ] `assessment.requires_evidence == false`
   - [ ] `assessment.requires_ordering == false`
   - [ ] `assessment.requires_phases == false`

2. **Program Generation**
   - [ ] Program has 1-2 steps (Shell + Reply)
   - [ ] No Plan step in program
   - [ ] No MasterPlan step in program
   - [ ] `program_matches_level(&program, ExecutionLevel::Action)` returns `Ok(())`

3. **Validation**
   - [ ] `program_is_overbuilt(&program, ExecutionLevel::Action) == false`
   - [ ] `program_is_underbuilt(&program, ExecutionLevel::Action) == false`

## Notes

This is the simplest execution level — a single command with no decomposition needed.

The ladder should recognize:
- No planning keywords ("plan", "strategy", "phases")
- No evidence chain needed (user knows what they want)
- Low risk (cargo test is safe, read-only operation)
- Low entropy (clear, unambiguous request)

## Related Scenarios

- ladder_002_task_read_summarize.md — Task level (evidence chain)
- ladder_006_overbuild_rejection.md — Rejects Plan for Action level
