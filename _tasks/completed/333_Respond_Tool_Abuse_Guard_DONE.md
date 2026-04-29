# Task 333: Respond Tool Abuse Guard

**Status:** pending
**Depends on:** Task 312 (stagnation finalization), Task 330 (consecutive loop guard)
**Session traces:** `s_1777399870_334678000`, `s_1777399876_334678000`
**Model:** `Huihui-Qwen3.5-4B-Claude-4.6-Opus-abliterated.Q6_K.gguf`

## Summary

Prevent small models from using the `respond` tool as a substitute for real tool calls. When a model calls `respond` repeatedly without any evidence-collecting tools in between, the system must detect the abuse, inject correction, and force evidence collection — rather than letting the model drain the iteration budget on empty status messages and then produce a useless final answer.

## Why

### The failure in `s_1777399870_334678000`

User asked: *"Are there any undo tasks?"*

The model's thinking block showed correct intent: *"I need to search the workspace... Let me start by searching for relevant terms in the codebase."* But instead of calling `search`, it called `respond` three times:

1. `respond(answer="Searching for undo-related tasks and functionality...")`  
2. `respond(answer="I'll search for undo tasks in the project.")`  
3. `respond(answer="Searching for undo tasks...")`  

Zero real tools. Zero evidence. The final answer model saw only these vague status messages and correctly concluded: *"Based on the evidence provided, no conclusion can be drawn."*

### Why existing stagnation detection failed

Task 312 and Task 330 hardened stagnation detection for real tools (shell, read, search), but `respond` was intentionally excluded:

| Line | File | Behavior |
|------|------|----------|
| `tool_loop.rs:505` | `tool_signal("respond")` returns `String::new()` | Respond signals are empty — never count toward stagnation |
| `tool_loop.rs:1014` | `// respond always continues the loop` | Respond never triggers finalization |
| `stop_policy.rs:290-291` | `register_signal("")` inserts empty string | First call registers as "new" signal → resets stagnation counter |

The model can call `respond` forever and the stagnation system won't notice. The only backstop is the iteration budget (15), and by the time that hits, all evidence slots are filled with useless status messages.

### Why this matters for Elma's philosophy

AGENTS.md Rule 7: *"The model is a given. The system must adapt to it."* A 4B model cannot reliably distinguish "announcing intent to search" from "actually searching." The system must provide guardrails that make the distinction for it.

## Root Cause Architecture

```
Model calls respond (no real tools used)
  → tool_signal returns "" (excluded from stagnation)
  → register_signal("") → true on first call (empty string is "new")
  → stagnation_runs resets to 0
  → respond always continues loop (line 1014)
  → next iteration: model calls respond again
  → repeat until iteration budget exhausted
  → finalization sees only respond messages as "evidence"
  → garbage answer
```

The `evidence_required` flag exists in `RouteDecision` (`src/types_core.rs:515`) and is computed by the classifier, but it is never checked inside the tool loop. It's an orchestration-level concept that has no effect on the raw tool loop path.

## Implementation Steps

### Step 1: Track consecutive respond calls without real tools

Add a counter to `StopPolicy` in `src/stop_policy.rs`:

```rust
// In struct StopPolicy:
consecutive_respond_calls: usize,
// In new():
consecutive_respond_calls: 0,
```

**Rules:**
- Increment when `respond` is called
- Reset to 0 when any evidence-collecting tool is called (shell, search, read, tool_search)
- When `consecutive_respond_calls >= 3`: inject a system correction message, then reset counter (give model a chance to correct)

### Step 2: Inject correction when respond abuse detected

In `src/tool_loop.rs`, after executing a `respond` tool call, check the counter:

```rust
if tc.function.name == "respond" {
    stop_policy.increment_respond_counter();
    if stop_policy.consecutive_respond_calls() >= 3 {
        messages.push(ChatMessage::simple(
            "user",
            "⚠️ You have called 'respond' 3 times without collecting any evidence. \
             You have not used search, read, shell, or any other tool to gather facts. \
             Your respond messages are status updates, not evidence. \
             Call a real tool now to answer the user's question, or reply with 'I cannot answer this.'"
        ));
        stop_policy.reset_respond_counter();
    }
}
```

### Step 3: Reset respond counter on real tool calls

In the tool execution section of `src/tool_loop.rs`, after executing any evidence-collecting tool:

```rust
if tc.function.name != "respond" && tc.function.name != "summary" && tc.function.name != "update_todo_list" {
    stop_policy.reset_respond_counter();
}
```

### Step 4: Wire `evidence_required` into the tool loop

In `src/tool_loop.rs`, when the tool loop receives its parameters, also receive `evidence_required: bool`. If `evidence_required == true` and the model calls `respond` before any real tool has run in this turn, reject the respond call:

```rust
if tc.function.name == "respond" && evidence_required && !stop_policy.has_real_tool_calls_this_turn() {
    // Replace respond with a correction message
    let correction = "You must collect evidence before answering.\n\
        Use search, read, or shell to gather facts. Do not call 'respond' yet.";
    result.content = correction;
}
```

The `evidence_required` flag is already computed by the classifier pipeline. It just needs to be threaded into `run_tool_loop()`.

### Step 5: Give respond a non-empty signal signature

In `src/tool_loop.rs:tool_signal()`, instead of returning an empty string:

```rust
"respond" => {
    let answer = parsed.get("answer")
        .or_else(|| parsed.get("content"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    // Truncate to first 40 chars to collapse similar messages
    let snippet = answer.chars().take(40).collect::<String>();
    format!("respond:{}", snippet)
}
```

This lets the stagnation detector distinguish *different* respond messages from identical ones. Two responds with identical text → stagnation. Two responds with different text → not necessarily stagnation.

### Step 6: Add respond-only stagnation path

Even with the above fixes, a model that persistently responds without real tools should eventually be stopped. Add a separate stagnation path:

```rust
// In StopPolicy:
consecutive_respond_only_turns: usize,

// In record_stagnation, when signal is a respond signal:
// Don't increment main stagnation_runs, but increment respond-only counter
// If respond-only counter >= 5, force-stop with a clear message
```

## Success Criteria

- [ ] After 3 consecutive `respond` calls with no real tools, system injects correction
- [ ] Correction message tells the model exactly what to do (call search/read/shell)
- [ ] If model heeds correction and calls a real tool, respond counter resets
- [ ] If model ignores correction and calls respond 5+ times, force-stop with useful message
- [ ] `evidence_required` flag blocks `respond` before evidence exists (when classifier says so)
- [ ] Empty-string signal no longer resets stagnation on first respond call
- [ ] `respond` calls with different messages are treated as different signals (not all identical)
- [ ] Session trace `s_1777399870_334678000` scenario (3 responds → no evidence → garbage answer) is prevented
- [ ] `cargo build` succeeds
- [ ] `cargo test` passes
- [ ] Unit tests: consecutive respond counter, evidence_required gating, signal differentiation

## Anti-Patterns To Avoid

- **Do NOT make respond a finalizing tool** — it must continue to function as interim status in normal use
- **Do NOT penalize legitimate respond use** — `shell → respond → shell → respond` is fine
- **Do NOT remove respond from the tool set** — it is a useful UX affordance
- **Do NOT add hardcoded keyword checks** — the guard must be behavioral (consecutive count), not content-based
- **Do NOT merge this with Task 330** — the consecutive respond guard is a distinct concern from command dedup/drift detection

## Reference: Files To Modify

| File | Changes |
|------|---------|
| `src/stop_policy.rs` | Add `consecutive_respond_calls`, `consecutive_respond_only_turns`, increment/reset/has_real_tool methods |
| `src/tool_loop.rs` | Check respond counter after execution, inject correction, reset on real tools, wire evidence_required, fix tool_signal for respond |
| `src/tool_loop.rs` (signature) | Add `evidence_required: bool` parameter to `run_tool_loop()` |
| `src/app_chat_loop.rs` | Thread `evidence_required` from `RouteDecision` into `run_tool_loop()` |
