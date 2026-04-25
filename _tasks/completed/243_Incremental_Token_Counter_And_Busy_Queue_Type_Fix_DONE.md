# 243: Incremental Transcript Token Counter And `await_with_busy_queue` Type Fix

## Status
`pending`

## Priority
High — Performance (O(n) scan per frame) + Reliability (double-Result pattern hides intent errors).

## Source
Code review findings H-7 and H-10. Two related issues in the orchestration and UI layers:

**H-10:** `estimate_transcript_tokens()` is called on every `draw()` invocation (~50ms interval). It scans every message in the transcript — O(n) per frame. On long sessions this causes visible stutter.

**H-7:** `await_with_busy_queue` wraps `Future<Output = T>` but is used with futures that return `Result<T>`, creating a `Result<Result<T>>`. At call-site, `.unwrap_or_else(|_|...)` on the outer layer silently swallows all intent-annotation errors. Intent failures are invisible.

## Objective
Fix both issues in a single surgical task since they share the `app_chat_loop.rs` / `ui_terminal.rs` boundary.

## Scope

### Part A — Incremental token counter (`src/ui/ui_terminal.rs`)

**1. Add a counter field to `TerminalUI`:**
```rust
pub(crate) struct TerminalUI {
    // ... existing fields ...
    transcript_token_estimate: u64, // incrementally maintained
}
```
Initialize to `0` in `TerminalUI::new()`.

**2. Update the counter in every message-push method:**
- `add_message`: `self.transcript_token_estimate += content.len() as u64 / 4;`
- `add_claude_message`: delegate to a `fn add_tokens_for_message(msg: &ClaudeMessage)` helper.
- `push_tool_finish`: add `command.len() / 4 + output.len() / 4`.
- `clear_messages`: reset to `0`.

**3. Replace the `estimate_transcript_tokens()` call in `draw_claude` with the stored value:**
```rust
let transcript_tokens_estimate = self.transcript_token_estimate;
```
Keep the old `estimate_transcript_tokens()` method but mark it `#[cfg(test)]` for use in tests only.

### Part B — Fix `await_with_busy_queue` double-Result (`src/app_chat_loop.rs`)

**1. Change the generic bound:**
```rust
async fn await_with_busy_queue<T, F>(
    tui: &mut TerminalUI,
    queued_inputs: &mut VecDeque<String>,
    future: F,
) -> Result<T>
where
    F: Future<Output = Result<T>>,  // ← was: Future<Output = T>
{
    tokio::pin!(future);
    loop {
        tokio::select! {
            result = &mut future => return result,  // ← was: return Ok(result)
            _ = tokio::time::sleep(Duration::from_millis(40)) => {
                tui.pump_ui()?;
                if let Some(queued) = tui.poll_busy_submission()? {
                    queued_inputs.push_back(queued);
                    tui.notify("Queued 1 message (will run after current response)");
                }
            }
        }
    }
}
```

**2. Update all call sites** — any `.await?` that was previously a double-unwrap is now a single clean `?`.

**3. Surface intent errors in the fallback:**
```rust
let rephrased_objective = await_with_busy_queue(
    &mut tui,
    &mut queued_inputs,
    annotate_user_intent(...),
)
.await
.unwrap_or_else(|e| {
    trace(&runtime.args, &format!("intent_annotation_failed error={e}"));
    line.to_string()
});
```

## Verification
- `cargo build` passes.
- `cargo test` passes.
- Long-session stress test: run 50+ turns and confirm no visible stutter in the footer token counter.
- Manual: deliberately fail the intent endpoint — verify error appears in trace log and Elma continues with the raw line.

## References
- `src/ui/ui_terminal.rs:573–608, 671–674` (estimate_transcript_tokens and draw_claude usage)
- `src/app_chat_loop.rs:21–42` (await_with_busy_queue)
- `src/app_chat_loop.rs:708–720` (intent annotation call site)
