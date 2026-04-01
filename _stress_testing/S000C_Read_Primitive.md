# Stress Test S000C: Read Primitive

## 1. The Test (Prompt)
"Find the README.md file within _stress_testing/_opencode_for_testing/ and summarize its core purpose."

## 2. Expected Behavior
- **Route:** PLAN (needs evidence)
- **Formula:** inspect_summarize_reply
- **Steps:** 2-4 (find + read + summarize + reply)

## 3. Success Criteria
- Agent finds and reads README.md
- Provides a concise summary
- Maximum 8 steps (step limit enforced)
- No duplicate/repeated steps

## 4. Common Failure Modes
- Wrong path (project root instead of test folder)
- Plan collapse (40+ identical steps)
