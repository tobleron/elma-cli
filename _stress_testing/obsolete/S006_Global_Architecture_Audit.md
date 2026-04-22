# Stress Test S006: Global Architecture Audit

## 1. The Test (Prompt)
"Perform an architecture audit of _stress_testing/_claude_code_src/ only. Sample broadly across that tree, score modules by complexity versus utility, and generate a report identifying the top 3 modules most in need of refactoring. Do not inspect or modify Elma's own src/ directory."

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
- No reads or edits outside `_stress_testing/_claude_code_src/`

## 4. Common Failure Modes
- Only auditing 1-2 files (laziness)
- Plan collapse (25+ identical grep commands)
- Math errors in scoring
- Auditing Elma's production code instead of the sandbox
