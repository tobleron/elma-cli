# Task 034: Predictive Failure Detection

## Context
Models often "hallucinate" or fail when context is near capacity or when the task is ambiguous.

## Objective
Implement predictive failure indicators:
- Monitor model entropy (if available via logprobs) or response "vibe" (length vs. complexity).
- Detect when context is nearing limits and proactively trigger a "Simplified" or "Safety-First" reasoning mode.

## Success Criteria
- Fewer catastrophic failures in long sessions.
- Graceful degradation of reasoning capability instead of hard crashes.
