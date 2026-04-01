# Stress Test S000A: Pure Conversational Baseline

## 1. The Test (Prompt)
"Hello Elma. Briefly explain your primary goal as a CLI agent."

## 2. Expected Behavior
- **Route:** CHAT
- **Formula:** reply_only
- **Steps:** 1 (Reply step only, no shell commands)

## 3. Success Criteria
- Agent responds with a concise explanation
- No shell commands executed
- No file searches performed
- Response is under 2000 characters

## 4. Common Failure Modes
- Over-orchestration: Generating shell steps for a simple chat question
- Hallucination: Claiming to be a different agent
