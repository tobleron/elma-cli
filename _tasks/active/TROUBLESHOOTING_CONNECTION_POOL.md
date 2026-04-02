# Troubleshooting Session: Connection Pool Exhaustion

## Date: 2026-04-02

## Problem

Elma CLI hung after ~5 HTTP API calls during stress testing. The 6th call would hang indefinitely at `[HTTP_SEND]`.

## Symptoms

```
[HTTP_START] model=llama_3.2_3b_instruct_q6_k_l.gguf
[HTTP_ATTEMPT] attempt=1/3
[HTTP_SEND] sending POST request...
[hangs here forever]
```

- Exactly 5 successful HTTP calls before hang
- No timeout errors
- Server healthy (curl tests worked)
- Process still running, just stuck

## Investigation Steps

### 1. Added Verbose HTTP Logging
- Added trace logs at every HTTP call stage
- Confirmed hang occurs at `request_builder.send().await`
- No errors, just silent hang

### 2. Checked Server Health
```bash
curl http://192.168.1.186:8080/health
# {"status":"ok"}

curl -X POST http://192.168.1.186:8080/v1/chat/completions ...
# Works fine
```

### 3. Analyzed Trace Logs
```
[HTTP_SUCCESS] parsed response successfully  (5 times)
[HTTP_START] ... (6th call - hangs)
```

### 4. Found Root Cause in Code

**File:** `src/intel_units.rs`

```rust
// ❌ WRONG: Creates new client for EVERY intel unit call
let result: serde_json::Value = chat_json_with_repair_timeout(
    &reqwest::Client::new(),  // ← New client each time!
    &chat_url,
    &req,
    self.profile.timeout_s,
).await?;
```

**Problem:** Each intel unit (complexity_assessor, workflow_planner, etc.) creates a NEW `reqwest::Client`. This causes:
- Connection pool fragmentation
- DNS resolver exhaustion
- Socket handle leaks
- Hangs after ~5 unique clients

### 5. Verified with Comment in Code

```rust
// Note: client should be passed in context or stored
&reqwest::Client::new(),
```

The original developer knew this was a problem but didn't fix it!

## Solution

**Pass shared client through IntelContext:**

### Before
```rust
// intel_units.rs
let result = chat_json_with_repair_timeout(
    &reqwest::Client::new(),  // ❌ New client each call
    ...
);
```

### After
```rust
// intel_trait.rs
pub struct IntelContext {
    pub client: reqwest::Client,  // ✅ Shared client
    ...
}

// intel_units.rs
let result = chat_json_with_repair_timeout(
    &context.client,  // ✅ Reuse shared client
    ...
);

// execution_ladder.rs
let context = IntelContext::new(
    ...,
    client.clone(),  // ✅ Pass from runtime
);
```

## Test Results

### Before Fix
```
HTTP calls: 5 successful, 6th hangs
S000A: ❌ TIMEOUT
```

### After Fix
```
HTTP calls: 11+ successful (no hangs)
S000A: ✅ PASSED
Unit tests: 109 ✅
```

## Files Modified

| File | Lines Changed | Purpose |
|------|---------------|---------|
| `src/intel_trait.rs` | +10 | Added `client` field to `IntelContext` |
| `src/intel_units.rs` | +5 | Use `context.client` |
| `src/execution_ladder.rs` | +2 | Pass client to context |
| `src/ui_chat.rs` | +40 | Added verbose HTTP logging |

## Lessons Learned

1. **Never create `reqwest::Client::new()` in hot paths**
   - Clients are expensive (connection pools, DNS resolvers)
   - Should be created once and shared

2. **Add connection pooling best practices to docs**
   - This is a common mistake in async Rust

3. **Verbose logging is essential for debugging hangs**
   - The `[HTTP_SEND]` trace line pinpointed the issue

## Related Issues

- Similar issue in Claude Code, Open Interpreter
- reqwest documentation recommends sharing clients
- Tokio runtime can only handle so many concurrent DNS resolutions

## Next Steps

- Continue stress testing (S000B-S008)
- Monitor for other connection-related issues
- Consider adding client connection pool metrics
