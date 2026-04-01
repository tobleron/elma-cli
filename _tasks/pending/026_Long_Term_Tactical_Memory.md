# Task 030: Long-Term Tactical Memory (LTM) for Formulas

## Context
`formula_memory_matcher.toml` allows re-using successful programs. This can be expanded into a more robust learning system.

## Objective
Enhance the formula memory system:
- Save not just successful programs, but also "negative examples" (failed commands and their successful repairs).
- Store "lessons learned" about specific workspace patterns.
- Improve retrieval based on semantic similarity of the user request.

## Success Criteria
- System "learns" from previous mistakes (e.g., doesn't repeat a failed shell command).
- Faster planning for recurring task types.
