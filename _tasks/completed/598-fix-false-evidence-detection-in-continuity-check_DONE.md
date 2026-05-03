# Task 598: Fix False Evidence Detection in Continuity Check

## Session Evidence
Session `s_1777824575_8073000`: The continuity tracker scored 0.78 (below 0.85 threshold) and triggered a retry. The culprit was `app_chat_loop.rs:988`:

```rust
let has_evidence = !step_results.is_empty() && step_results.iter().any(|r| r.ok);
```

For the direct tool-calling pipeline (used for most requests), `step_results` is always empty — the tool loop executes everything internally without populating `step_results`. So `has_evidence` is always `false`, causing `check_final_answer` to fire:

```rust
if !has_evidence && original_norm.split_whitespace().count() > 3 {
    self.add_checkpoint(
        "finalization",
        ContinuityVerdict::Drifted(
            "Final answer has no supporting evidence for non-trivial request"
        ),
        ...
    );
}
```

The model gathered ~200KB of evidence across 18 tool calls (cat, glob, ls), but the continuity tracker was blind to all of it.

## Problem
`has_evidence` is derived from `step_results`, which only the old execution-step framework populates. The direct tool-calling pipeline collects evidence through the evidence ledger, not through step results. This causes every tool-calling answer to be flagged as lacking evidence, triggering unnecessary continuity retries.

Even after Task 597 (lightweight retry), this false trigger wastes a model call and potentially degrades the answer quality.

## Solution
Derive `has_evidence` from the evidence ledger instead of from `step_results`:

```rust
let has_evidence = if !step_results.is_empty() {
    step_results.iter().any(|r| r.ok)
} else {
    // Direct tool-calling: check evidence ledger
    crate::evidence_ledger::get_session_ledger()
        .map(|l| l.entries_count() > 0)
        .unwrap_or(false)
};
```

In `src/app_chat_loop.rs`, change line 988. The evidence ledger tracks every tool output with an entry count — this is the authoritative signal for whether evidence was gathered.
