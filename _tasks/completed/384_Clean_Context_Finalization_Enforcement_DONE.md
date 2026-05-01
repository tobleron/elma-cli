# Task 384: Clean-Context Finalization Enforcement

**Status:** Pending
**Priority:** MEDIUM
**Estimated effort:** 2-3 days
**Dependencies:** None
**References:** objectives.md "How We Get There" #2, AGENTS.md Rule 3

## Problem

Per objectives.md: "Clean-context finalization — final answers never leak internal state, stop reasons, or error messages." The ratatui terminal final answer must also be plain text by default, not markdown-first output.

Current behavior (from session `s_1777658707_425792000` final answer):
```
=== Final Answer ===
Solution
Given Evidence:
- The current time is 17:35:06 (5:35 PM)
Analysis: ...
```

The final answer contains internal framing ("Given Evidence:", "Analysis:", "Verification:") that leaked from the tool-calling pipeline. Final answers should be clean — just the answer, formatted for the user. The "Solution/Analysis/Evidence" framing is an internal trace artifact, not a user-facing answer.

## Objective

Enforce that final answers:
1. Are free of internal state (no "Given Evidence:", "Analysis:", "Verification:" prefixes)
2. Are free of stop reasons and error messages
3. Are free of thinking/reasoning artifacts
4. Are formatted as direct user-facing answers
5. Pass through a finalizer intel unit that strips or rewrites leaked artifacts
6. Render as clean plain text in the terminal unless the user explicitly requested a markdown artifact

## Implementation Plan

### Phase 1: Final Answer Sanitizer

Create `src/final_answer.rs`:

```rust
/// Strips internal artifacts from final answers before display
pub(crate) fn sanitize_final_answer(raw: &str) -> String {
    let mut cleaned = raw.to_string();

    // Strip known framing patterns
    let patterns = [
        "=== Final Answer ===",
        "## Solution",
        "**Given Evidence:**",
        "**Analysis:**",
        "**Answer:**",
        "Verification:",
        "Step 1:",
        "Step 2:",
    ];

    for pattern in &patterns {
        if let Some(pos) = cleaned.find(pattern) {
            // Extract only the user-facing portion after the framing
            // (context-dependent — use finalizer intel unit for safety)
        }
    }

    cleaned
}
```

### Phase 2: Finalizer Intel Unit

Create `src/intel_units/intel_units_final_cleaner.rs`:

A focused intel unit that:
- Takes raw final answer + original user intent
- Returns clean, user-facing answer only
- Removes all internal framing, evidence wrappers, and stop reasons
- System prompt: "Rewrite the following internal answer into a clean, direct response to the user. Remove all evidence formatting, analysis sections, step headers, and internal metadata. Output only the response text."

### Phase 3: Enforcement Gate

In `src/app_chat_loop.rs` after `resolve_final_text` (line 1028):

```rust
let final_text = sanitize_final_answer(&raw_answer);

// If sanitization removed too much, run through finalizer intel unit
if final_text.len() < raw_answer.len() / 3 {
    let cleaned = run_final_cleaner(&runtime, &raw_answer, line).await?;
    final_text = cleaned;
}
```

### Phase 4: Pattern Blocklist

Add a compile-time blocklist of phrases that must never appear in final answers:
- "Given Evidence"
- "Analysis:"
- "=== Final Answer ==="
- "Stop reason"
- "Tool loop"
- "Stagnation"
- Error messages (stack traces, "failed to", etc.)
- Markdown-only wrappers that do not improve terminal readability

If any of these appear in the final text, the finalizer intel unit is automatically invoked.

## Files to Create/Modify

| File | Action |
|------|--------|
| `src/final_answer.rs` | CREATE — sanitizer + pattern blocklist |
| `src/intel_units/intel_units_final_cleaner.rs` | CREATE — LLM finalizer intel unit |
| `src/app_chat_loop.rs` | MODIFY — wire sanitizer + finalizer into answer path |

## Verification

```bash
cargo build
cargo test final_answer
cargo test sanitize
```

**Manual**: Send any query and verify the displayed answer:
1. Contains no "Given Evidence", "Analysis", "=== Final Answer ===" headers
2. Is formatted as a direct user-facing response
3. Does not leak internal stop reasons or error messages
