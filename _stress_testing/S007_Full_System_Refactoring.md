# Stress Test S007: Full System Refactoring

## 1. The Test (Prompt)
"Standardize the logging style across _stress_testing/_opencode_for_testing/. Find all files using println! or console.log. Create a utility module to wrap these calls. Refactor all identified files to use the new utility."

## 2. Expected Behavior
- **Route:** PLAN (multi-file refactor)
- **Formula:** inspect_edit_verify_reply
- **Steps:** 8-12 (find + create utility + edit multiple + verify)

## 3. Success Criteria
- Agent identifies all logging calls
- Creates utility module
- Refactors all files consistently
- Maximum 12 steps (absolute limit enforced)
- Edit content under 500 characters per edit

## 4. Common Failure Modes
- Context collapse (forgetting overall objective)
- Content explosion (200+ lines in single edit)
- Plan collapse (40+ steps)
