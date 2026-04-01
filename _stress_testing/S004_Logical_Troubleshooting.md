# Stress Test S004: Logical Troubleshooting

## 1. The Test (Prompt)
"There is a reported issue where JSON responses are missing the 'id' field. Find the code responsible for parsing JSON responses and implement a robust fallback."

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

## 4. Common Failure Modes
- Hallucinating bugs that don't exist
- Failing to write working test code
- Plan collapse (40+ steps)
