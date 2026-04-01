# Task 001: Enable Reflection For All Tasks (CRITICAL)

## Priority
**P0 - CRITICAL** (Highest accuracy gain, minimal code change)

## Status
**ACTIVE** - Ready for implementation

## Problem
Reflection is currently skipped for `complexity=DIRECT` tasks via `should_skip_intel()`. This is when hallucination is MOST likely - simple tasks where the model is overconfident.

**Evidence from session s_1774963523_616156000:**
```
trace: reflection_skipped complexity=direct
[CLASSIFY] speech=CHAT route=SHELL (entropy=1.24)
...
critic_parse_error=No valid or repairable JSON object found
```

The model executed `date` command successfully, but critics hallucinated failure. Reflection would have caught this BEFORE execution.

## Objective
Remove the `should_skip_intel()` check for reflection. Run reflection for ALL tasks regardless of complexity.

## Implementation

### Files to Modify
1. `src/app_chat_core.rs` - Remove reflection skip logic
2. `src/orchestration_helpers.rs` - Remove or deprecate `should_skip_intel()`

### Changes
```rust
// BEFORE (app_chat_core.rs)
let skip_intel = should_skip_intel(&complexity);
if !skip_intel {
    match reflect_on_program(...) { ... }
}

// AFTER
// Always run reflection - it catches hallucination even for simple tasks
match reflect_on_program(...) {
    Ok(reflection) => {
        if !reflection.is_confident {
            // Revise program before execution
            program = revise_program(program, reflection).await?;
        }
    }
    Err(error) => {
        trace_verbose(runtime.verbose, &format!("reflection_failed error={}", error));
        // Continue with original program - reflection is advisory
    }
}
```

## Acceptance Criteria
- [ ] Reflection runs for ALL tasks (DIRECT, INVESTIGATE, MULTISTEP, OPEN_ENDED)
- [ ] Reflection failures don't block execution (advisory only)
- [ ] Trace shows `reflection_confidence=...` for all tasks
- [ ] No increase in execution time >20%
- [ ] Reduction in critic hallucination rate

## Expected Impact
- **+30% accuracy** on simple tasks (fewer hallucination-induced retries)
- **-25% retry rate** (issues caught before execution)
- **Minimal latency** (~200ms for reflection call)

## Dependencies
- None (reflection module already exists)

## Verification
- `cargo build`
- `cargo test`
- Test with "what is current time" - should reflect before executing
- Check trace for `reflection_confidence` on DIRECT tasks

## Architecture Alignment
- ✅ Accuracy and reliability first (P0 priority)
- ✅ Minimal changes for maximum gain
- ✅ Preserves existing architecture (no breaking changes)
