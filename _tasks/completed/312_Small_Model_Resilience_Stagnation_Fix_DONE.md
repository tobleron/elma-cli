# Task 312: Small Model Resilience — Stagnation Bottleneck Fix

**Status**: fixes implemented  
**Session traces**: `s_1777307653_887096000`, `s_1777309268_156609000`  
**Model**: `Huihui-Qwen3.5-4B-Claude-4.6-Opus-abliterated.Q6_K.gguf`

## Summary

Multiple bugs compounded across two sessions:
1. HTTP truncation false positives caused 3x latency on every API call
2. Permission gate blocked harmless `2>/dev/null` commands
3. Stagnation finalization pipeline failed on small models
4. Persistent shell captured PS1 prompt as command output (date/pwd issue)
5. Status thread not visible during tool loop (pending_draw cleared after first draw)<tool_call>` markup even when asked to produce plain text

Additionally, a pre-existing HTTP truncation bug causes **every single API call** to retry 3 times (3x latency, 3x token waste).

## Root Cause Analysis

### Bug A: HTTP truncation detection — every response treated as truncated

**File**: `src/ui/ui_chat.rs:35-44`

```rust
if !last_char.is_whitespace()
    && !matches!(
        last_char,
        '.' | '!' | '?' | '"' | '\'' | ')' | ']' | '}' | '`' | '~'
    )
{
    if trimmed.len() > 50 {
        return true;
    }
}
```

Any response >50 chars that doesn't end with whitespace/punctuation is classified as truncated. This means:
- Intent helper responses ending in a word (e.g. `"...the current time"`) → truncated
- Tool call responses ending with `</tool_call>` (last char `>`) → truncated  
- Basically every meaningful response → 3 HTTP attempts instead of 1

**Evidence**: Session trace shows `[HTTP_RETRY] response appears truncated` on every single API call (intent, turn summary, tool loop model calls, finalization attempts).

### Bug B: Permission gate `>` check is too broad

**File**: `src/shell_preflight.rs:646-666`

The mutation prefix list includes `">"` which matches any command containing `>`. This catches `2>/dev/null` (harmless stderr redirection to trash) and blocks commands that operate on protected paths.

**The blocked command**:
```bash
cd /Users/r2/elma-cli && find src/ -type f \( -name "*.rs" -o -name "*.toml" \)
  | while read f; do if ! grep -q "^$f\|^src/$f$" .gitignore 2>/dev/null; then echo "$f"; fi; done | wc -l
```

This is purely read-only (filtering .rs/.toml files to exclude gitignored ones), but `2>/dev/null` triggers the `>` mutation flag, and `src/` is protected → blocked.

### Bug C: Stagnation finalization pipeline fails on small models

**File**: `src/tool_loop.rs:733-765`

When stagnation is detected:
1. A user message is appended: "Tool loop appears repetitive. Finalize now..."
2. `request_final_answer_without_tools()` is called with `tools: None`, `temperature: 0.0`
3. The model keeps generating `<tool_call>` markup because its conversation history is full of tool calls
4. `final_answer_needs_retry()` detects `<tool_call>` → retries with "Return plain terminal text only"
5. Model STILL generates `<tool_call>` → falls back to `build_fallback_from_recent_tool_evidence()`
6. Fallback only grabs first line of last 3 tool outputs → weak answer like "src/workspace.rs\n220\n220"

**Why small models fail**: A 4B Qwen fine-tune cannot context-switch from tool-calling mode to text mode when the conversation history is saturated with tool calls. The model has learned "when I see tool results, I should generate the next tool call." Breaking this pattern requires either:
- A fresh context that doesn't contain tool-call history
- A different model call with higher temperature
- Skipping the model entirely and formatting evidence directly

### Bug D: Status thread never invoked (from previous session)

**File**: `src/ui/ui_terminal.rs:429-440`

`start_status`/`complete_status`/`clear_status` are defined but never called from the chat loop.

---

## Fix Plan

### Fix A: HTTP truncation (smallest, highest impact)

Add `>` and `:` to the allowed-ending list. Also check the API's `finish_reason` field — if it's `"stop"`, the response is complete regardless of the last character.

### Fix B: Permission gate `>`

For the `>`/`>>` mutation flags, exclude redirects to `/dev/null`:
- `2>/dev/null`
- `>/dev/null 2>&1`

Or better: only flag `>` when it's not a stderr/stdout redirect to `/dev/null`.

### Fix C: Stagnation finalization pipeline

Replace the current "add a message and hope" approach with:

1. When stagnation triggers, collect ALL tool results from the current turn
2. Format them as a compact evidence block
3. Create a **fresh prompt** with:
   - The user's original request
   - The evidence block
   - A clear instruction to produce a terminal answer (no tools available in this mode)
4. Call the model with temperature=0.2 (not 0.0) to encourage variation
5. If the model still fails, use the evidence block directly as the answer (skip model)

This is a "reset the context" approach rather than an "append and hope" approach.

### Fix D: Status thread wiring

Call `start_status("Working...")` at the start of tool loop execution, `complete_status("Done")` when the final answer is produced, `clear_status()` when a new turn starts.

### Bug E: Persistent shell captures PS1 prompt as output

**File**: `src/persistent_shell.rs:21-74`

The persistent shell runs `stty -echo` to suppress command echo, but the shell **prompt (PS1)** is still written to the PTY after each command. When reading output until the marker line, the prompt line is captured as part of the output.

**Evidence**: `date && pwd` returned `r2@Artos-MacBook-Air elma-cli %` (the zsh prompt) instead of the actual date and path.

**Fix**: Set `PS1=""` and `PS2=""` in the shell environment before spawning.

### Bug F: Status thread invisible during tool loop

**File**: `src/ui/ui_terminal.rs:756-765`

`pump_ui()` is called during the tool loop to keep the UI responsive, but `draw()` skips rendering unless `pending_draw` is true. After the first draw, `pending_draw` is set to false and never re-set during the tool loop.

**Fix**: Check if `status_thread.is_working()` in `pump_ui()` and force `pending_draw = true` while the spinner is active.

---

## Evidence Timeline (s_1777307653_887096000)

**Turn 1** — "what time is it and which directory are we on?"
- Intent: "The user is asking for the current time" (note: incompletely parsed)
- Tool: `date && pwd` → output was prompt text only (shell issue)
- Answer: returned time from prompt, got directory from workspace

**Turn 2** — "how many lines of code is this project?"
- Intent: annotated correctly
- Iter 1: `find /Users/r2/elma-cli -name '.gitignore' | head -1` → found .gitignore
- Iter 2: Generated the `while read f ... grep -q ... 2>/dev/null` command
- Iter 3: PREFLIGHT BLOCKED (mutation flag false positive)
- Iter 4: `find src/ ... | wc -l` → 220 (got count)
- Iter 5: `find src/ ... | wc -l` → 220 (stagnation run 1 — same command)
- Iter 6: `find src/ ... | sort` → `src/workspace.rs` (output unexpectedly truncated to 1 file)
- Iter 7: `find src/ ... | sort` → 220 (stagnation run 1 — same command but different output)
- Iter 8: `find src/ ... | sort` → 220 (stagnation threshold reached → `repeated_same_command`)

**Finalization**:
- Attempt 1: Model generated `<tool_call>` instead of text
- Attempt 2: Model still generated `<tool_call>` (force_plain_text=true)
- Fallback: "I couldn't finalize cleanly, but here are the most recent grounded findings: src/workspace.rs, 220, 220"

## Evidence Timeline (s_1777309268_156609000)

**Turn 1** — "hi"
- Intent: annotated correctly
- Tool: `respond` → greeting

**Turn 2** — "what time is it and what path are we on?"
- Tool: `date && pwd` → output was `r2@Artos-MacBook-Air elma-cli %` (PS1 prompt, not actual date/pwd)
- Answer: model hallucinated time, got directory from workspace (not from shell output)

**Turn 3** — "calculate lines of source code under src directory"
- Iter 1: `find src ... | wc -l` → 220
- Iter 2: same command → 220 (stagnation run 1)
- Iter 3: `find src ... -exec wc -l {} + | tail -1` → 220
- Iter 4: same command → 75557 total (stagnation run 1)
- Iter 5: same command → repeated_same_command stop
- Finalization: new clean-context approach → "**Total lines of source code: 75557**" (success!)

## Fixes Applied

| Fix | File | Description |
|-----|------|-------------|
| A | `src/ui/ui_chat.rs` | Removed over-aggressive truncation heuristic; added `finish_reason="stop"` bypass |
| B | `src/shell_preflight.rs` | Added `sanitize_null_redirects()` to exclude `2>/dev/null` from mutation detection |
| C | `src/tool_loop.rs` | New `request_final_answer_from_evidence()` with clean context + temp=0.2 |
| D | `src/orchestration_core.rs`, `src/app_chat_loop.rs` | Wired `start_status`/`complete_status`/`clear_status` |
| E | `src/persistent_shell.rs` | Set `PS1=""`, `PS2=""` to suppress prompt capture |
| F | `src/ui/ui_terminal.rs` | Force `pending_draw=true` in `pump_ui()` while status thread is working |

## Success Criteria

- [x] HTTP truncation detection no longer retries valid responses
- [x] Permission gate allows harmless `2>/dev/null` stderr suppression
- [x] Stagnation finalization produces useful answers without tool-call loops
- [x] `cargo build` passes, existing tests pass
- [x] Persistent shell returns actual command output (not prompt)
- [x] Status thread visible during tool loop execution
