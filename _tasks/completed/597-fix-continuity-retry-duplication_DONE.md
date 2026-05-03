# Task 597: Fix Continuity Retry Orchestration Duplication

## Session Evidence
Session `s_1777824575_8073000`: For the request "read all docs and compare with source code," Elma ran **two full tool-calling pipelines** (23+ tool calls total):

| Cycle | Tool Calls | What |
|-------|-----------|------|
| 1 | workspace_info → glob → ls → read×3(fail) → cat docs×7 → cat src×4 → summary | Full discovery |
| 2 | workspace_info → glob → ls → read×3(fail) → cat docs×5 → cat src×5 → summary | EXACT same discovery |

Cycle 2 was triggered by the continuity guard at `app_chat_loop.rs:1012`:
```rust
if continuity_tracker.alignment_score < 0.85 && !already_retried {
    // Re-runs run_tool_calling_pipeline from scratch
}
```

The retry re-discovered the workspace, re-globbed all .md files, re-listed docs/, re-failed the read tool 3 times, and re-catted the same files. Zero context carried forward from Cycle 1.

## Problem
The continuity guard (`Task 498`) calls `run_tool_calling_pipeline` again with a `[continuity_retry]` prompt. This starts a **fresh tool loop** with empty `tool_outcomes` map, empty evidence ledger, fresh workspace discovery. Every tool call from Cycle 1 is repeated identically in Cycle 2.

This wastes:
- 50%+ of iterations (task actually needed ~7 cat calls, got 20+)
- Model tokens on repeated discovery
- User's wall-clock time
- Context window on duplicate evidence

## Solution
**Option A (Recommended): Light retry**
When continuity retry triggers, do NOT call `run_tool_calling_pipeline`. Instead, append a re-prompt message to `runtime.messages` and re-run only `run_tool_loop` with the existing conversation context. The model can improve its answer without re-discovering anything.

Implementation in `app_chat_loop.rs:1012-1046`:
```rust
if continuity_tracker.alignment_score < 0.85 && !already_retried {
    let retry_msg = format!(
        "[continuity_retry]\nThe previous answer may not fully address your request.\nOriginal request: {}\nIssue detected: {}\n\nPlease provide a more complete answer focused on what was asked. Do NOT call any tools — just improve your answer based on the evidence you already collected.",
        line, gap_reason
    );
    runtime.messages.push(ChatMessage::simple("user", &retry_msg));
    // Re-run tool_loop with existing messages (it will likely call summary immediately)
    // ... run_tool_loop continues from existing state
}
```

**Option B: Pass tool outcomes map**
If running the full pipeline is unavoidable, pass the previous `tool_outcomes` map and `last_evidence_summary` to the retry loop. This would skip duplicate tool calls via the existing dedup gate (`tool_loop.rs:1225-1232`).

**Option C: Skip retry when budget was sufficient**
If the first loop used > 50% of its iteration budget, skip continuity retry entirely. The model had ample opportunity to produce a complete answer.
