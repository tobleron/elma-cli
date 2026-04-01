# Stress Test S008: Workflow Endurance (The Marathon)

## 1. The Test (Prompt)
"Perform a complete documentation audit of _stress_testing/_opencode_for_testing/. Map every directory. For every .go file, identify the main functions. Compare implementation against README.md. Create AUDIT_REPORT.md with your findings. Summarize the biggest inconsistency found."

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

## 4. Common Failure Modes
- Infinite loops (searching same folder repeatedly)
- Hangs (orchestration loop fails to produce next step)
- Goal drift (forgetting the summary requirement by turn 10+)
