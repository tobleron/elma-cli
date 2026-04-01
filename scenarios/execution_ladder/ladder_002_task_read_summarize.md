# Scenario: Ladder 002 - Task Level (Read and Summarize)

## Suite
execution_ladder

## File
ladder_002_task_read_summarize.md

## Speech Act
INSTRUCTION

## Workflow
INSPECT

## Mode
INSPECT

## Route
PLAN

## Expected Formula
inspect_summarize_reply

## Expected Execution Level
Task

## User Message
read AGENTS.md and summarize it

## Expected Behavior

### Level Selection
- **Level:** Task
- **Rationale:** Bounded outcome requiring evidence chain (Read → Summarize → Reply)
- **Complexity:** INVESTIGATE
- **Risk:** LOW

### Program Shape
- **Steps:** 2-4 (Read + Summarize + Reply, or Read + Reply with inline summary)
- **No Plan step** — Task level doesn't need explicit planning structure
- **No MasterPlan step** — Task level doesn't need strategic decomposition

### Expected Program Structure
```json
{
  "objective": "read AGENTS.md and summarize it",
  "steps": [
    {
      "type": "read",
      "id": "r1",
      "path": "AGENTS.md",
      "purpose": "Read the AGENTS.md file",
      "success_condition": "File content loaded successfully"
    },
    {
      "type": "summarize",
      "id": "s1",
      "instructions": "Summarize the key points of AGENTS.md",
      "purpose": "Extract main points from file content",
      "depends_on": ["r1"]
    },
    {
      "type": "reply",
      "id": "r2",
      "instructions": "Present the summary to the user",
      "purpose": "Deliver summary to user"
    }
  ]
}
```

### Validation Rules
- ✅ Task level allows 2-8 steps
- ✅ Task level rejects Plan step
- ✅ Task level rejects MasterPlan step
- ✅ Program must have Reply step
- ✅ Evidence chain (Read → Summarize) is appropriate

## Acceptance Criteria

1. **Level Assessment**
   - [ ] `assess_execution_level()` returns `ExecutionLevel::Task`
   - [ ] `assessment.level == Task`
   - [ ] `assessment.requires_evidence == true` (needs to read file)
   - [ ] `assessment.requires_ordering == true` (Read before Summarize)
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

This is a classic Task-level request:
- Bounded outcome (summarize one file)
- Evidence chain required (must read before summarizing)
- No explicit planning structure needed
- Single session, single objective

The ladder should recognize:
- Evidence gathering needed (read file)
- Transformation needed (summarize)
- No planning keywords
- Low risk (read-only operation)

## Related Scenarios

- ladder_001_action_cargo_test.md — Action level (single operation)
- ladder_003_task_evidence_chain.md — Task with search+read
- ladder_006_overbuild_rejection.md — Rejects Plan for Task level
