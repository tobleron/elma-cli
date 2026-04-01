# Stress Test S006: Global Architecture Audit

## 1. The Test (Prompt)
"Perform an audit of the entire src/ directory. Score every file based on its complexity vs utility. Generate a report identifying the top 3 modules most in need of refactoring."

## 2. Expected Behavior
- **Route:** PLAN (complex analysis task)
- **Formula:** inspect_reply or plan_reply
- **Steps:** 8-12 (survey files + analyze + report)

## 3. Success Criteria
- Agent samples multiple src/ files
- Applies scoring criteria
- Identifies top 3 refactor candidates
- Maximum 12 steps (absolute limit enforced)
- No duplicate steps (>50% duplicates = fail)

## 4. Common Failure Modes
- Only auditing 1-2 files (laziness)
- Plan collapse (25+ identical grep commands)
- Math errors in scoring
