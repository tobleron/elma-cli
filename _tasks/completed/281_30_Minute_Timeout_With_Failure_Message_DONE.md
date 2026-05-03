# Task 281: 30-Minute Timeout for Tool Loop with Clear Failure Message

## Problem

Local small models (like Qwen3.5-4B) are slow and sessions were timing out silently or hanging indefinitely. Users needed:
1. A predictable timeout limit
2. Clear feedback on what happened when timeout occurred
3. Metrics showing time spent and progress made

## Solution

Added 30-minute timeout to tool loop with detailed failure message.

### Changes

**1. `ToolLoopResult` struct** (`tool_loop.rs`):
```rust
pub(crate) struct ToolLoopResult {
    pub(crate) final_answer: String,
    pub(crate) iterations: usize,
    pub(crate) tool_calls_made: usize,
    pub(crate) stopped_by_max: bool,
    pub(crate) stop_outcome: Option<StopOutcome>,
    pub(crate) total_elapsed_s: f64,       // NEW: time tracking
    pub(crate) timeout_reason: Option<String>, // NEW: timeout cause
}
```

**2. Timeout check at loop start**:
```rust
let total_timeout = Duration::from_secs(30 * 60);
let loop_start = Instant::now();

loop {
    // Check 30-minute timeout
    let elapsed = loop_start.elapsed();
    if elapsed > total_timeout {
        return Ok(ToolLoopResult {
            final_answer: format!(
                "⏱️ **Timeout After {:.1} Minutes**\n\n\
                 The task was cancelled due to exceeding the 30-minute time limit.\n\n\
                 **Time spent:** {:.1} minutes\n\
                 **Iterations completed:** {}\n\
                 **Tool calls made:** {}\n\n\
                 **Cause:** Slow model response time (local model)\n\n\
                 Try simplifying the request or breaking it into smaller steps.",
                elapsed_mins, elapsed_mins,
                stop_policy.iteration(),
                stop_policy.total_tool_calls()
            ),
            // ... other fields
        });
    }
    // ...
}
```

### Timeout Message

When timeout occurs, user sees:
```
⏱️ **Timeout After 32.5 Minutes**

The task was cancelled due to exceeding the 30-minute time limit.

**Time spent:** 32.5 minutes
**Iterations completed:** 12
**Tool calls made:** 8

**Cause:** Slow model response time (local model)

Try simplifying the request or breaking it into smaller steps.
```

## Implementation Details

- Uses `std::time::Instant` to track total elapsed time
- Checks timeout at the START of each loop iteration (before starting new model request)
- Gracefully terminates with structured result
- Includes progress metrics (iterations, tool calls)
- Clear human-readable message

## Completion Criteria

- [x] 30-minute timeout implemented
- [x] Clear failure message with time spent
- [x] Progress metrics (iterations, tool calls) included
- [x] Build passes
- [x] Task created