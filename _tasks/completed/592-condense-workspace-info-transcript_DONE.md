# Task 592: Condense Workspace Information in Terminal Transcript

## Session Evidence
Session `s_1777823506_810966000`: `workspace_info` was called 4 times in a single session (turn 1 × 2, turn 2 cycle 1, turn 2 cycle 2). Each call dumps ~250 lines into the terminal transcript — dominated by the directory tree listing (200+ lines). The user's terminal is flooded with repeated directory trees for no value.

## Problem
`workspace_info` output is already provided to the model in the system prompt (via `build_tool_calling_system_prompt`). When the tool is called at runtime, the full output is also emitted to the terminal transcript, creating massive visual noise. The transcript is the user's primary view — it should show concise, actionable information, not raw system data.

## Solution
1. In `emit_tool_result()` (tool_calling.rs line 215), truncate `workspace_info` tool output for the TRANSCRIPT (not for the model):
   - Keep only: root path, project type, git status, guidance files loaded
   - Strip: the full directory tree listing (200+ lines)
   - Replace with: `Directory tree available in evidence; summary: N entries across M directories`
2. The full output still goes to the model context and evidence ledger
3. Only the transcript preview is condensed
4. Apply the same truncation to `shell` output > 200 lines for transcript purposes
