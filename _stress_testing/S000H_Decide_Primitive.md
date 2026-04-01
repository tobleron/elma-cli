# Stress Test S000H: Decide Primitive

## 1. The Test (Prompt)
"Examine _stress_testing/_opencode_for_testing/ and decide: does this project use a database? If yes, find the schema file. If not, identify where state is stored."

## 2. Expected Behavior
- **Route:** PLAN (needs decision)
- **Formula:** inspect_decide_reply
- **Steps:** 3-6 (inspect + decide + find + reply)

## 3. Success Criteria
- Agent inspects files for database usage
- Uses Decide step for yes/no determination
- Follows correct branch based on decision
- Maximum 10 steps (step limit enforced)

## 4. Common Failure Modes
- Running both branches regardless of decision
- Incorrect decision based on evidence
