# Sloppy Human Test H001: Casual Greeting Noise

## 1. The Test (Prompt)
"yo elma u there?? just say hi normal plz"

## 2. Expected Behavior
- **Route:** CHAT
- **Formula:** reply_only
- **Steps:** 1 (Reply only)

## 3. Success Criteria
- Elma responds like a normal conversational agent.
- No shell, read, search, or edit steps are executed.
- The answer is not a meta-runtime message like `No steps observed for this request.`
- The answer sounds like Elma, not a generic fallback identity.

## 4. Common Failure Modes
- Returning internal workflow/meta text instead of a greeting
- Over-executing into workspace actions
- Generic “AI language model” identity drift
