# 153 Graceful Shutdown And Panic Recovery

## Summary
Implement graceful shutdown and panic recovery for stability.

## Reference
- OpenCode: `internal/logging/logger.go`, `cmd/root.go`

## Implementation

### 1. Panic Recovery
File: `src/panic.rs` (new)
```rust
pub fn recover_panic<F, R>(component: &str, f: F) -> Option<R>
where
    F: FnOnce() -> R,
{
    std::panic::catch_unwind(std::panic::catch_assertions(f))
        .map_err(|info| {
            // Log to file
            // Stack trace
        })
}
```

### 2. Graceful Shutdown
File: `src/shutdown.rs` (new)
```rust
pub struct Shutdown {
    cancel: broadcast::Sender<()>,
}

impl Shutdown {
    pub fn new() -> Self { ... }
    pub fn shutdown(&self) {
        // Cancel all watchers
        // Wait for goroutines
        // Close LSP clients
        // Flush logs
    }
}
```

### 3. App Lifecycle
File: `src/app.rs`
- Add `shutdown()` method
- `App::run()` uses `defer shutdown()`
- Cleanup on Ctrl+C

### 4. Logging
File: `src/logging.rs` (new)
- Persist logs to `~/.elma-cli/logs/`
- Log levels: trace, debug, info, warn, error
- Panic logs with stack traces

## Verification
- [ ] `cargo build` passes
- [ ] Panic recovery works
- [ ] Shutdown cleans up properly