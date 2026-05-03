# Fix Complexity Assessment Bypass

## Problem
In session `s_1777807006_86051000`, the user provided a sweeping, open-ended prompt: "read all docs then compare them to source code to tell me what needs to be updated."
According to `trace_debug.log`, the system routed this directly to `tool_calling: direct model planning (no Maestro)`.

This is a direct violation of **Rule 4a (Complexity is the Main Gate)** and **Rule 4 (Small-Model-Friendly Decomposition)**. Tasks requiring directory-wide reads and cross-comparisons must be assessed as `MULTISTEP` (Depth 3) or `OPEN_ENDED` (Depth 4+), triggering the Maestro work graph. Instead, a constrained 4B model was tasked with solving the entire problem in a single deep prompt, leading to failure.

## Required Actions
1. **Audit Complexity Assessor:** Investigate why the routing/intent annotation layer evaluated this prompt as simple enough for direct planning.
2. **Harden Routing Logic:** Ensure that prompts containing multiple verbs ("read... then compare... tell me") or bulk target requests ("all docs", "source code") forcefully escalate complexity to Maestro.
3. **Add Fallback Guard:** If `direct model planning` detects an excessive number of read operations in its first loop, it should abort and dynamically re-escalate to `MULTISTEP`.
