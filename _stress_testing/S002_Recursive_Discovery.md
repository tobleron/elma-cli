# Stress Test S002: Recursive Discovery

## 1. The Test (Prompt)
"Perform a recursive scan of _stress_testing/_opencode_for_testing/. Map the directory structure and identify the top 3 largest files by line count."

## 2. Expected Behavior
- **Route:** PLAN (multi-step investigation)
- **Formula:** inspect_reply
- **Steps:** 4-8 (find + count + sort + reply)

## 3. Success Criteria
- Agent maps directory structure
- Identifies top 3 largest files
- Maximum 12 steps (absolute limit enforced)
- No duplicate steps (>50% duplicates = fail)

## 4. Common Failure Modes
- Plan collapse (40+ identical steps)
- Duplicate step loops
