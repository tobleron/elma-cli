# Stress Test S000D: Search Primitive

## 1. The Test (Prompt)
"Search _stress_testing/_opencode_for_testing/ for any TODO comments and list the files where they occur."

## 2. Expected Behavior
- **Route:** PLAN (needs evidence)
- **Formula:** inspect_reply
- **Steps:** 2-4 (grep search + reply)

## 3. Success Criteria
- Agent uses grep or search to find TODOs
- Lists files with line numbers
- Maximum 8 steps (step limit enforced)
- No duplicate/repeated commands

## 4. Common Failure Modes
- Searching wrong directory
- Plan collapse (40+ identical grep commands)
