# Task 498: Fix Turns Completed Always Zero

**Status:** pending
**Priority:** medium
**Primary surfaces:** `src/app.rs:76,101`, `src/app_chat_loop.rs`
**Related issues:** All sessions in `session_status.json` show `turns_completed: 0` regardless of actual user prompts

## Objective

Track the actual number of user turns completed in each session and write it to `session_status.json` / `session.json` instead of the current hardcoded `0`.

## Problem

`app.rs:101` hardcodes `turns_completed` to `0`:
```rust
let _ = crate::write_session_status(&session_root, "completed", 0, None, None);
```

The same on error at line 105:
```rust
crate::write_session_status(&session_root, "error", 0, None, Some(&e.to_string()));
```

This means **every session** shows `0` turns completed. For example, session `s_1777744940_287543000` had 2 user turns ("hi" + "read all docs") but `session_status.json:6` reports `"turns_completed": 0`. The `session.json` `turn_summaries` object correctly records `turn_0` for the greeting, but the status counter doesn't match.

## Implementation Plan

1. Add `turn_count: u32` to `AppRuntime` struct at `app.rs:53`:
   ```rust
   pub(crate) struct AppRuntime {
       ...
       pub(crate) turn_count: u32,
   }
   ```

2. Initialize to `0` in `app_bootstrap.rs` or wherever `AppRuntime` is constructed.

3. Increment in `app_chat_loop.rs` each time a user prompt is received and the tool loop begins processing it. Locate the call site where a user message is enqueued or where `tool_loop` is invoked, and add:
   ```rust
   runtime.turn_count += 1;
   ```

4. Update `app.rs:99-107` to pass `runtime.turn_count`:
   ```rust
   match &result {
       Ok(()) => {
           let _ = crate::write_session_status(&session_root, "completed", runtime.turn_count, None, None);
       }
       Err(e) => {
           let _ = crate::write_session_status(&session_root, "error", runtime.turn_count, None, Some(&e.to_string()));
       }
   }
   ```

## Success Criteria

- New session `session_status.json` shows `turns_completed` matching the actual number of user prompts (e.g., `2` for a session with two prompts)
- Existing sessions remain unchanged (only affects new sessions)
- `cargo build && cargo test -- session_error` passes
