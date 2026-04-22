# Stress Test S001: Connectivity and UI Formatting

## 1. The Test (Prompt)
"Hello Elma. This is a formatting-only baseline inside _stress_testing/: briefly state your role as a local CLI agent, then list your current model id and base URL in a compact plain-text table. Do not run commands, inspect files, or modify anything inside or outside _stress_testing/."

## 2. Expected Behavior
- **Route:** CHAT
- **Formula:** reply_only
- **Steps:** 1 (Reply only, no shell commands)

## 3. Success Criteria
- Agent correctly identifies its configuration
- Response is formatted as a table
- Maximum 2000 characters (truncation enforced)
- No infinite token repetition
- No shell, read, search, or edit steps

## 4. Common Failure Modes
- Token repetition bugs (same token 200+ times)
- Over-orchestration (shell steps for a chat question)
- Inventing workspace facts
