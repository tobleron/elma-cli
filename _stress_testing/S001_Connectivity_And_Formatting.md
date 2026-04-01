# Stress Test S001: Connectivity and UI Formatting

## 1. The Test (Prompt)
"Hello, identify your current configuration, model, and the number of active profiles you have loaded. Format your response in a clean table."

## 2. Expected Behavior
- **Route:** CHAT
- **Formula:** reply_only
- **Steps:** 1 (Reply only, no shell commands)

## 3. Success Criteria
- Agent correctly identifies its configuration
- Response is formatted as a table
- Maximum 2000 characters (truncation enforced)
- No infinite token repetition

## 4. Common Failure Modes
- Token repetition bugs (same token 200+ times)
- Over-orchestration (shell steps for a chat question)
