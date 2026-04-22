# Stress Test S007: Full System Refactoring

## 1. The Test (Prompt)
"Standardize the logging style across _stress_testing/_claude_code_src/ only. Find a small, coherent subset of files that use inconsistent logging patterns, create one shared wrapper utility under _stress_testing/_claude_code_src/, and refactor only that verified subset to use the new utility. Do not attempt a repo-wide rewrite and do not touch files outside _stress_testing/."

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
- All writes remain inside `_stress_testing/_claude_code_src/`

## 4. Common Failure Modes
- Context collapse (forgetting overall objective)
- Content explosion (200+ lines in single edit)
- Plan collapse (40+ steps)
- Unbounded repo-wide refactor instead of an incremental subset
