# Task 600: Improve Continuity Retry Message to Include Evidence Context

## Session Evidence
Session `s_1777824575_8073000`: After Task 597, the continuity retry now sends a lightweight text-only call with the full conversation context. But the retry message itself is still generic:

```
"[continuity_retry]\nThe previous answer may not fully address your request.\nOriginal request: {line}\nIssue detected: {gap}\n\nPlease improve your answer based on the evidence already collected. Do NOT call any tools."
```

The `gap_reason` from `continuity_tracker.gap()` for this session was just the last checkpoint reason, which was something like `"final_len=2454 has_evidence=false"`. This is internal metadata, not a useful instruction to the model.

## Problem
After Task 598 fixes `has_evidence=false`, continuity retries will become rarer. But when they do fire (genuine misalignment), the retry message provides very little actionable guidance. The model gets a cryptic gap reason and a generic "improve your answer" request.

## Solution
Enhance the continuity retry message with a structured summary of what the model already did, what evidence it gathered, and what specifically was missing or misaligned:

```rust
let retry_msg = format!(
    "[continuity_retry]\n\
    The previous answer may not fully address your request.\n\n\
    Original request: {line}\n\n\
    What you did: collected evidence from {ev_count} tool calls \
    ({tool_list}).\n\n\
    Issue detected: The answer appears to have a gap vs the request.\n\
    Specifically: {gap}\n\n\
    Please provide a more complete answer. Do NOT call any tools — use \
    the evidence you already gathered. Reference specific files and \
    findings in your improved answer.",
    ev_count = evidence_count,
    tool_list = tool_summary,
    gap = human_readable_gap,
    line = line
);
```

In `src/app_chat_loop.rs`, modify the continuity retry block (Task 597 section). Derive `evidence_count` from the evidence ledger. Derive `tool_summary` from the conversation context. Derive `human_readable_gap` by translating the raw gap reason into plain English.
