# Scenario: Ladder 003 - Task Level with Evidence Chain (Find Symbol Definition)

## Suite
execution_ladder

## File
ladder_003_task_evidence_chain.md

## Speech Act
INSTRUCTION

## Workflow
INSPECT

## Mode
INSPECT

## Route
PLAN

## Expected Formula
inspect_reply

## Expected Execution Level
Task

## User Message
find where fetch_ctx_max is defined

## Expected Behavior

### Level Selection
- **Level:** Task
- **Rationale:** Evidence chain required (Search → Read → Reply)
- **Complexity:** INVESTIGATE
- **Risk:** LOW

### Program Shape
- **Steps:** 2-4 (Search + Read + Reply)
- **No Plan step** — Task level doesn't need explicit planning structure
- **No MasterPlan step** — Task level doesn't need strategic decomposition

### Expected Program Structure
```json
{
  "objective": "find where fetch_ctx_max is defined",
  "steps": [
    {
      "type": "search",
      "id": "s1",
      "query": "fetch_ctx_max",
      "paths": ["src/"],
      "purpose": "Search for fetch_ctx_max definition",
      "success_condition": "Search returns matching file paths"
    },
    {
      "type": "read",
      "id": "r1",
      "path": "src/path/to/file.rs",
      "purpose": "Read the file containing the definition",
      "depends_on": ["s1"],
      "success_condition": "File content loaded successfully"
    },
    {
      "type": "reply",
      "id": "r2",
      "instructions": "Show the definition location and context",
      "purpose": "Present findings to user"
    }
  ]
}
```

### Validation Rules
- ✅ Task level allows 2-8 steps
- ✅ Task level rejects Plan step
- ✅ Task level rejects MasterPlan step
- ✅ Program must have Reply step
- ✅ Evidence chain (Search → Read) is appropriate

## Acceptance Criteria

1. **Level Assessment**
   - [ ] `assess_execution_level()` returns `ExecutionLevel::Task`
   - [ ] `assessment.level == Task`
   - [ ] `assessment.requires_evidence == true` (needs to search and read)
   - [ ] `assessment.requires_ordering == true` (Search before Read)
   - [ ] `assessment.requires_phases == false`

2. **Program Generation**
   - [ ] Program has 2-4 steps
   - [ ] No Plan step in program
   - [ ] No MasterPlan step in program
   - [ ] `program_matches_level(&program, ExecutionLevel::Task)` returns `Ok(())`

3. **Validation**
   - [ ] `program_is_overbuilt(&program, ExecutionLevel::Task) == false`
   - [ ] `program_is_underbuilt(&program, ExecutionLevel::Task) == false`

## Notes

This is a Task-level request with a multi-step evidence chain:
1. Search for the symbol
2. Read the file containing the definition
3. Reply with findings

The ladder should recognize:
- Evidence gathering needed (search codebase)
- Multiple sources may be consulted
- No explicit planning structure needed
- Bounded scope (find one symbol)

## Related Scenarios

- ladder_001_action_cargo_test.md — Action level (single operation)
- ladder_002_task_read_summarize.md — Task with read+summarize
- ladder_004_plan_refactor.md — Plan level (explicit planning)
