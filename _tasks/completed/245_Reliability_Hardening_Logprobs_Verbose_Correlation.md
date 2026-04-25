# 245: Reliability Hardening — Logprobs Profile Field, Bootstrap Verbose Default, Request Correlation IDs

## Status
`pending`

## Priority
Medium — Reliability and observability: discarded probe result, wrong verbose default, untraceable concurrent requests.

## Source
Code review findings M-15, L-20, and the architecture note on request correlation.

**M-15:** `probe_logprobs_support` result in `models_api.rs` is discarded (`let _ = ...`). The `ModelBehaviorProfile` struct has no `supports_logprobs` field. The probe is wasted I/O that informs nothing.

**L-20:** `AppRuntime` is constructed with `verbose: true` hardcoded in `app_bootstrap_core.rs:269`, ignoring `args.verbose`. Verbose mode should be opt-in, not always-on.

**Architecture:** All HTTP requests to the LLM endpoint share one `reqwest::Client` with no request IDs. When two concurrent calls fail (e.g. during tuning), their trace log entries are indistinguishable. A `X-Request-Id` header would allow correlation.

## Objective
Three targeted fixes in a single low-risk task:
1. Persist `supports_logprobs` in `ModelBehaviorProfile` or remove the wasted probe.
2. Fix the `verbose: true` hardcode to respect `args.verbose`.
3. Inject a per-request UUID header into all LLM HTTP calls.

## Scope

### Fix 1 — Logprobs probe (`src/models_api.rs`, `src/types_api.rs`)

**Option A (preferred):** Remove the wasted probe call until the result is consumed somewhere:
```rust
// Delete lines 452–454:
// let _ = probe_logprobs_support(client, chat_url, model_id).await.ok();
```

**Option B:** Add `supports_logprobs: bool` to `ModelBehaviorProfile` in `types_api.rs` and persist the result:
```rust
pub struct ModelBehaviorProfile {
    // ... existing fields ...
    pub supports_logprobs: bool,
}
```
Then in `ensure_model_behavior_profile`:
```rust
let logprobs = probe_logprobs_support(client, chat_url, model_id).await.unwrap_or(false);
let profile = ModelBehaviorProfile { ..., supports_logprobs: logprobs };
```

### Fix 2 — Verbose default (`src/app_bootstrap_core.rs`)
Line 269: change `verbose: true` → `verbose: args.verbose`.

### Fix 3 — Request correlation IDs (`src/models_api.rs` or shared HTTP helper)
Add a lightweight helper that generates a short request ID:
```rust
fn new_request_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    format!("elma-{:08x}", nanos)
}
```
Inject it into every `client.post(...).json(req)` call:
```rust
client
    .post(chat_url.clone())
    .header("X-Request-Id", new_request_id())
    .json(req)
    .send()
```

For a proper solution, consider integrating with the existing `tracing` subscriber to propagate a span ID instead.

## Verification
- `cargo build` passes.
- `cargo test` passes.
- `rg 'verbose: true' src/app_bootstrap_core.rs` returns zero matches.
- With `--verbose` flag: verbose output appears. Without it: silent by default.
- With `--debug-trace`: request IDs appear in the trace log alongside LLM call entries.
- `cargo audit` still clean.

## References
- `src/models_api.rs:452–454` (probe_logprobs_support)
- `src/app_bootstrap_core.rs:269` (verbose: true hardcode)
- `src/models_api.rs:202–225` (probe_chat_completion_raw — HTTP send site)
