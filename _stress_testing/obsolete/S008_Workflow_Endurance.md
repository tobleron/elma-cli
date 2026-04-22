# Stress Test S008: Workflow Endurance (The Marathon)

## 1. The Test (Prompt)
"Perform a documentation audit inside _stress_testing/_opencode_for_testing/ only. Map the major directories, inspect a representative subset of the Go files, compare the implementation against README.md, create _stress_testing/_opencode_for_testing/AUDIT_REPORT.md with your findings, and summarize the single biggest inconsistency you found. Stay inside _stress_testing/ for all reads and writes."

## 2. Expected Behavior
- **Route:** PLAN (multi-step endurance test)
- **Formula:** inspect_reply or plan_reply
- **Steps:** 10-12 (map + analyze + compare + create report + summarize)

## 3. Success Criteria
- Agent completes all 5 sub-tasks
- Creates AUDIT_REPORT.md
- Identifies biggest inconsistency
- Maximum 12 steps (absolute limit enforced)
- No infinite loops or hangs
- No reads or writes outside `_stress_testing/`

## 4. Common Failure Modes
- Infinite loops (searching same folder repeatedly)
- Hangs (orchestration loop fails to produce next step)
- Goal drift (forgetting the summary requirement by turn 10+)
- Escaping the sandbox boundary
