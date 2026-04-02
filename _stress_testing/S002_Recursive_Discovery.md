# Stress Test S002: Recursive Discovery

## 1. The Test (Prompt)
"Inspect only _stress_testing/_opencode_for_testing/. Map its directory structure and identify the top 3 largest source files by line count. Do not inspect or modify files outside _stress_testing/."

## 2. Expected Behavior
- **Route:** PLAN (multi-step investigation)
- **Formula:** inspect_reply
- **Steps:** 4-8 (find + count + sort + reply)

## 3. Success Criteria
- Agent maps directory structure
- Identifies top 3 largest files
- Maximum 12 steps (absolute limit enforced)
- No duplicate steps (>50% duplicates = fail)
- No reads, searches, or edits outside `_stress_testing/`

## 4. Common Failure Modes
- Plan collapse (40+ identical steps)
- Duplicate step loops
- Escaping the stress-testing sandbox
