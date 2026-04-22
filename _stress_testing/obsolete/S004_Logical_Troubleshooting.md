# Stress Test S004: Logical Troubleshooting

## 1. The Test (Prompt)
"Inside _stress_testing/_claude_code_src/ only, investigate a hypothetical issue where some parsed JSON responses may be missing an 'id' field. Find one parsing path that is vulnerable to missing-field handling, implement a robust fallback, and verify the change locally. Do not inspect or modify Elma's own src/ directory."

## 2. Expected Behavior
- **Route:** PLAN (troubleshooting task)
- **Formula:** inspect_edit_verify_reply
- **Steps:** 6-10 (find code + understand + fix + verify + reply)

## 3. Success Criteria
- Agent identifies JSON parsing code
- Implements fallback logic
- Tests the fix
- Maximum 12 steps (absolute limit enforced)
- No duplicate steps
- No edits outside `_stress_testing/_claude_code_src/`

## 4. Common Failure Modes
- Hallucinating bugs that don't exist
- Failing to write working test code
- Plan collapse (40+ steps)
- Touching Elma's own runtime code instead of the stress sandbox
