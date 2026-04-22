# Stress Test S000E: Sequential Logic

## 1. The Test (Prompt)
"In _stress_testing/_opencode_for_testing/, find a function definition in one file, then search for every location where that function is called."

## 2. Expected Behavior
- **Route:** PLAN (multi-step)
- **Formula:** inspect_reply
- **Steps:** 3-6 (find function + search calls + reply)

## 3. Success Criteria
- Agent identifies a function name
- Searches for call sites
- Lists locations where function is called
- Maximum 10 steps (step limit enforced)
- No duplicate steps (>50% duplicates = fail)

## 4. Common Failure Modes
- Forgetting the function name between steps
- Duplicate step loops
