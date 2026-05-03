# Task 561: Complexity-Aware Dynamic Iteration Budget

## Session Evidence
In session `s_1777820401_246730000`:
- User asked: "read ALL docs and compare with source code" (OPEN_ENDED, reading 50+ files)
- System assigned: max_iterations=3 (DIRECT, for simple fact lookup)
- Result: 0 docs read, task completely failed

The trace shows every task gets the same 3 iteration budget regardless of difficulty:
```
trace: tool_loop: starting max_iterations=3 stagnation_threshold=8 timeout=30m
```

## Problem
The iteration budget is a fixed constant (3) that doesn't scale with task complexity. A "hi" greeting (DIRECT, needs 1-2 iterations) gets the same budget as "read all docs" (OPEN_ENDED, needs 20+). This guarantees failure on multi-step tasks.

## Solution
After complexity assessment determines the task type, assign iteration budgets:
| Complexity | Max Iterations | Notes |
|------------|---------------|-------|
| DIRECT | 3 | Simple fact, single operation |
| INVESTIGATE | 6 | File reading, searching |
| MULTISTEP | 12 | Multi-file analysis, code changes |
| OPEN_ENDED | 20+ | Large-scale analysis, exploration |

Also implement:
1. A "task progress" signal that estimates completion percentage based on items done / items needed
2. If budget is halfway exhausted and progress is <30%, warn the user and ask if they want to continue
3. Allow the model to request more iterations if it demonstrates forward progress
4. Minimum iterations = ceil(estimated_steps * 1.5) with a floor of 3
