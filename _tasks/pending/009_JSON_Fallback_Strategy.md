# Task 009: JSON Fallback Strategy (Unified Error Handling)

## Priority
**P0 - CRITICAL** (Must be implemented BEFORE Task 008 Phase 1)

## Status
**CORE INFRASTRUCTURE COMPLETE** - Unified module ready for integration

## Completed

### Unified Module: `src/json_error_handler.rs`
- ✅ **Circuit Breaker** - Prevents cascade failures
  - Opens after 5 consecutive failures
  - 60-second cooldown before recovery attempt
  - Requires 3 successes to close from half-open state
  - States: Closed (normal) → Open (degraded) → HalfOpen (testing)

- ✅ **Safe Defaults** - Fallback values for all components
  - `default_critic_verdict()` - Returns "ok" with explanatory reason
  - `default_outcome_verdict(exit_code)` - Uses exit code as ground truth
  - `default_sufficiency_verdict()` - Assumes ok when unavailable
  - `default_formula_selection(route)` - Route-appropriate defaults
  - `default_scope(objective)` - Minimal safe scope
  - `default_fallback_program(line, route)` - User-friendly fallback
  - `default_decomposition_result(objective)` - Safe goal structure
  - `default_workflow_plan(objective)` - DIRECT complexity default
  - `default_complexity_assessment()` - DIRECT/LOW/reply_only
  - `default_route_decision()` - CHAT route default

- ✅ **User-Facing Errors** - Never show raw JSON errors
  - `user_facing_json_error_message()` - Helpful message

- ✅ **Metrics & Logging**
  - `log_fallback_usage()` - Track fallback usage by component
  - `record_json_failure()` / `record_json_success()` - Global tracking
  - `is_degraded_mode()` - Check system state

- ✅ **Global Instance**
  - `get_error_handler()` - Singleton access
  - Thread-safe with Mutex

- ✅ **Tests** - All 42 tests pass including:
  - Circuit breaker opens after threshold
  - Circuit breaker resets on success
  - Default verdicts are safe
  - User-facing messages never show raw errors

## Remaining Work

### Integration (Systematic Rollout)
- ⏳ Add fallback at every JSON parse call site (~30 locations)
- ⏳ Wrap `build_program()` with error handling
- ⏳ Wrap critic/verification calls with fallbacks
- ⏳ Add user-facing error handling in `run_chat_loop()`

### Testing
- ⏳ Integration tests for fallback chains
- ⏳ Chaos testing with random JSON failures
- ⏳ Verify zero crashes in 1000+ failure injection tests

## Usage Pattern

```rust
// At every JSON parse call site:
match parse_json_result {
    Ok(valid_json) => {
        record_json_success(&args);
        process_valid_json(valid_json)
    }
    Err(error) => {
        record_json_failure(&args, "component_name");
        
        // Try deterministic repair first
        match repair_semantic_errors(&partial_data) {
            Ok(repaired) => process_valid_json(repaired),
            Err(_) => {
                // Use safe default
                log_fallback_usage(&args, "component", "parse_failure", "default");
                get_safe_default_for_component()
            }
        }
    }
}

// In user-facing code:
match process_user_message(...) {
    Ok(response) => println!("Elma: {}", response),
    Err(_) => println!("Elma: {}", user_facing_json_error_message()),
}
```

## Acceptance Criteria
- [x] Unified error handler module created
- [x] Circuit breaker implemented and tested
- [x] Safe defaults for all components
- [x] User-facing error messages (never raw errors)
- [x] Global tracking instance
- [x] All tests pass (42/42)
- [ ] Every JSON parse call site has fallback
- [ ] User never sees raw JSON error messages
- [ ] Circuit breaker triggers after 5 consecutive failures
- [ ] Fallback rate <5% in normal operation
- [ ] Zero crashes due to JSON parsing in 1000+ test runs
