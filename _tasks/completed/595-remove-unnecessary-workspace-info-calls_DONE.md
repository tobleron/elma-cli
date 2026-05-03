# Task 595: Remove Unnecessary Workspace Info Calls

## Session Evidence
Session `s_1777823506_810966000`: workspace_info was called at:
1. Turn 1, iteration 1 — discover workspace
2. Turn 1, iteration 3 — called AGAIN even though already gathered
3. Turn 2, cycle 1, iteration 1 — called AGAIN (fresh turn, but system prompt already has workspace context)
4. Turn 2, cycle 2, iteration 1 — called AGAIN (cross-cycle restart)

The system prompt (built by `build_tool_calling_system_prompt`) already includes the full workspace brief from the session's `workspace_brief.txt`. The model should NOT need to call `workspace_info` to discover the workspace — it's already in its initial context.

## Problem
The model calls `workspace_info` as its first action in nearly every turn, wasting the first iteration on rediscovering information it already has. This was useful for the early version when workspace context wasn't in the system prompt, but now it's already available. Each call wastes 1 iteration and dumps 250 lines into the terminal transcript.

## Solution
1. Add a prominent instruction in the system prompt (prompt_core.rs) that says:
   `Your first message already includes the workspace root and directory structure. Do NOT call workspace_info unless you need to refresh git status or discover new files not in the brief.`
2. When `workspace_info` IS called, include in its result a notice: `(workspace info provided at session start)`
3. Track `workspace_info_called_this_turn: bool` in the tool loop and if called more than once per turn, log but don't deduplicate (the model may have valid reason to refresh)

Implementation location: `src/prompt_core.rs` (add instruction) and `src/tool_loop.rs` (add tracking).
