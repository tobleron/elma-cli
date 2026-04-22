# 154 Retry With Exponential Backoff

## Summary
Implement retry with exponential backoff for API calls.

## Reference
- OpenCode: `internal/llm/provider/openai.go` - retry logic

## Implementation

### 1. Retry Config
File: `src/retry.rs` (new)
```rust
pub struct RetryConfig {
    pub max_retries: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub jitter: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 8,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            jitter: 0.1,
        }
    }
}
```

### 2. Retry Function
```rust
pub async fn retry<T, E, F, Fu>(
    config: &RetryConfig,
    operation: F,
) -> Result<T, E>
where
    F: Fn() -> Fu,
    Fu: Future<Output = Result<T, E>>,
{
    let mut attempts = 0;
    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) if attempts >= config.max_retries => return Err(e),
            Err(e) => {
                // Check if retryable
                if !is_retriable(&e) { return Err(e); }
                attempts += 1;
                sleep(calculate_delay(attempts, config)).await;
            }
        }
    }
}
```

### 3. Retriable Errors
```rust
pub fn is_retriable<E: Display>(error: &E) -> bool {
    // 429 Too Many Requests
    // 500 Internal Server Error
    // 502 Bad Gateway
    // 503 Service Unavailable
    // 504 Gateway Timeout
    // Connection errors
}
```

### 4. Integration
File: `src/api_client.rs`
- Wrap LLM API calls with retry
- Respect `Retry-After` header

## Verification
- [ ] `cargo build` passes
- [ ] Retry respects max_retries
- [ ] Jitter prevents thundering herd