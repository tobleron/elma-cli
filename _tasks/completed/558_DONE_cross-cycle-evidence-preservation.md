# Task 558: Cross-Cycle Evidence Preservation

## Session Evidence
In session `s_1777820401_246730000`, after the first 3-iteration cycle failed (iteration_limit_reached), the system started a SECOND 3-iteration cycle from scratch. Both cycles performed the exact same steps:
- Cycle 1: workspace_info → ls docs → read(fail)
- Cycle 2: workspace_info → ls docs → read(fail)

The evidence from cycle 1 was fully available on disk (`e_001_raw.txt`, `e_002_raw.txt`) but was not injected into cycle 2's initial context. Cycle 2 wasted 2 of its 3 iterations rediscovering information it already had.

## Problem
When a tool_loop restarts due to budget exhaustion, the new loop's initial context doesn't include the evidence already gathered. This leads to duplicate work and wasted iteration budget. The cumulative wasted budget (4 of 6 iterations repeating already-done work) made it impossible to reach the actual read step.

## Solution
1. When restarting a tool_loop, inject a "Previously gathered evidence" section into the first model call
2. Include the evidence summary from the prior cycle (what files were listed, what was found)
3. Mark these as ALREADY DONE so the model doesn't repeat them
4. Format: `[PREVIOUSLY GATHERED] You already: (1) listed workspace root, (2) listed docs/ directory (53 items). Do NOT repeat these steps. Continue from: read the documentation files.`
5. Track cumulative budget across cycles, not per-cycle
