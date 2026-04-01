# Stress Test S003: Multi-File Refactor

## 1. The Test (Prompt)
"Choose a simple utility function in _stress_testing/_opencode_for_testing/. Rename it to something more descriptive and update all its call sites. Verify by searching for the old name."

## 2. Expected Behavior
- **Route:** PLAN (multi-step refactor)
- **Formula:** inspect_edit_verify_reply
- **Steps:** 6-10 (find + read + edit multiple + verify + reply)

## 3. Success Criteria
- Agent identifies a function to rename
- Updates definition and all call sites
- Verifies no old name remains
- Maximum 12 steps (absolute limit enforced)
- No duplicate steps

## 4. Common Failure Modes
- Editing wrong codebase (src/ instead of test folder)
- Missing call sites
- Plan collapse (40+ steps)
