# Task 556: Complexity Pre-Assessment as Mandatory Gate

## Session Evidence
In session `s_1777820401_246730000`, the user asked:
> "read all docs and compare with source code to tell me exactly what needs to change"

The trace shows:
```
trace: planning_source=maestro ladder_level=Task
trace: tool_calling: direct model planning (no Maestro)
trace: tool_loop: starting max_iterations=3
```

The task was assessed as `ladder_level=Task` (equivalent to DIRECT complexity), with 3 max iterations, no work graph decomposition. This was objectively wrong — reading ALL docs and comparing with source code is OPEN_ENDED complexity.

## Problem
Complexity assessment exists in code but is bypassed. The "direct model planning" path skips the complexity gate entirely and assigns a flat iteration budget. There's no enforcement that the complexity assessment must run first.

## Solution
1. Make `AssessingComplexity` a required agent state transition before `ExecutingToolLoop`
2. If complexity is DIRECT (simple fact lookup, single file read, etc.), allow 3 iterations
3. If complexity is INVESTIGATE or higher, scale iteration budget and enable work graph decomposition
4. The complexity assessment must itself be a fast model call (token-light)
5. Reject any task that can't be properly classified — don't default to DIRECT
