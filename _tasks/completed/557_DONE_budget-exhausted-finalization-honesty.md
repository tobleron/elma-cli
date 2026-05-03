# Task 557: Budget-Exhausted Finalization Honesty

## Session Evidence
In session `s_1777820401_246730000`, after 6 total iterations (3+3 across two cycles), zero docs were read, zero comparisons were made. The final answer was:
```
Based on the evidence gathered:
- ## Workspace Root
- docs/ (53 item(s))
```
This is semantically a lie — the answer implies the task was completed by summarizing gathered "evidence" when in reality the agent failed to do ANY of what was asked.

## Problem
When the agent runs out of iteration budget before completing the user's task, the finalization step generates a confident-sounding "Based on the evidence gathered..." summary that masks the failure. The user has no way to know the task wasn't actually completed.

## Solution
1. The stop reason must be surfaced in the final answer text
2. Template: `[Budget exhausted: I read 0/53 docs before hitting the iteration limit. To complete this task I would need more iterations. Here's what I found so far: ...]`
3. The finalization prompt must treat "budget exhausted" as a distinct case from "task completed"
4. The summary tool should NOT be used for budget-exhausted finalization — use a "progress_report" that clearly communicates what was done vs what wasn't done
5. If zero meaningful work was done (0 docs read for a "read all docs" task), the final answer should be explicitly "I didn't complete this task" — not a vague evidence summary
