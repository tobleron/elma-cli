# Stress Test S000F: Select Primitive

## 1. The Test (Prompt)
"In _stress_testing/_opencode_for_testing/, identify three potential files that could be the main application logic. Select the most likely candidate and explain your reasoning."

## 2. Expected Behavior
- **Route:** PLAN (needs selection)
- **Formula:** inspect_decide_reply or inspect_reply with Select step
- **Steps:** 2-4 (list files + select + reply)

## 3. Success Criteria
- Agent lists candidate files
- Uses Select step to choose one
- Explains reasoning for selection
- Maximum 8 steps (step limit enforced)

## 4. Common Failure Modes
- Skipping Select step
- Selecting non-existent files
