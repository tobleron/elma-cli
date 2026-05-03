# 570 — Implement Bounded Retry with Exponential Backoff for API Calls

- **Priority**: Medium
- **Category**: Error Handling
- **Depends on**: None
- **Blocks**: None

## Problem Statement

LLM API calls throughout the codebase have inconsistent retry behavior:

1. **`orchestration_core.rs`**: `orchestrate_instruction_once()` retries up to 2 times with deterministic settings (lines 266-306)
2. **`tool_loop.rs`**: Streaming request falls back to non-streaming on error (lines 1007-1044), but no retry on transient errors
3. **`ui_chat.rs`**: Appears to have timeout-based retry
4. **No standard backoff**: Each retry uses hardcoded constants, no exponential backoff, no jitter

Transient errors from local LLM servers (connection refused, timeout, 502/503) are common with small self-hosted models.

## Why This Matters for Small Local LLMs

Local models running on consumer hardware frequently hit:
- Timeouts when generating long responses
- Connection drops during streaming
- 503 errors when the server is under load
- GPU OOM errors requiring server restart

Without proper retry, each transient failure becomes a permanent failure from the user's perspective.

## Current Behavior

```rust
// orchestration_core.rs — hardcoded 2 retries, no backoff
const MAX_RETRIES: u32 = 2;
for attempt in 0..=MAX_RETRIES {
    // ...
}

// tool_loop.rs — single fallback, no retry
Err(error) => {
    // Try non-streaming fallback once
    let mut fallback_req = req;
    fallback_req.stream = false;
    let resp = await_with_busy_input(/* ... */).await?;
}
```

## Recommended Target Behavior

Create a unified retry policy:

```rust
pub struct RetryPolicy {
    pub max_retries: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
    pub jitter: bool,
    pub retry_on: Vec<RetryCondition>,
}

pub enum RetryCondition {
    StatusCode(u16),           // e.g., 429, 502, 503
    Timeout,
    ConnectionError,
    RateLimit,
}

impl RetryPolicy {
    pub fn for_streaming() -> Self { /* 3 retries, 1s base, 30s max */ }
    pub fn for_one_shot() -> Self { /* 2 retries, 500ms base, 10s max */ }
    pub fn for_critical() -> Self { /* 5 retries, 2s base, 60s max */ }
}
```

## Source Files That Need Modification

- `src/llm_provider.rs` — Add retry wrapper
- `src/tool_loop.rs` — Use unified retry for streaming requests
- `src/orchestration_core.rs` — Use unified retry (or remove with Task 550)
- `src/ui_chat.rs` — Use unified retry

## New Files/Modules

- `src/retry.rs` — `RetryPolicy`, retry executor

## Acceptance Criteria

- All LLM API calls use unified retry policy
- Exponential backoff with jitter for transient errors
- Configurable per call type (streaming, one-shot, critical)
- Retry attempts logged to trace
- Max total retry time is capped (no infinite retry)

## Risks and Migration Notes

- Retry may hide persistent errors (bad API key, wrong endpoint). Limit retries on 4xx errors except 429.
- Retry during streaming is complex — some servers don't support idempotent retry for streaming. Fall back to non-streaming on second retry.
