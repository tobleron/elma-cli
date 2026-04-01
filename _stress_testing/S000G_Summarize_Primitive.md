# Stress Test S000G: Summarize Primitive

## 1. The Test (Prompt)
"Read the README.md in _stress_testing/_opencode_for_testing/ and create a 3-bullet point executive summary."

## 2. Expected Behavior
- **Route:** PLAN (needs evidence + summarization)
- **Formula:** inspect_summarize_reply
- **Steps:** 2-4 (read + summarize + reply)

## 3. Success Criteria
- Agent reads README.md
- Uses Summarize step
- Output is exactly 3 bullet points
- Maximum 8 steps (step limit enforced)

## 4. Common Failure Modes
- Not actually summarizing (just repeating text)
- Wrong number of bullet points
