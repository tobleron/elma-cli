# Task 740: Replace Char-Based Token Estimation With Model-Aware Token Counting

## Type

Reliability / Accuracy / Context Management

## Severity

High

## Scope

Token counting, status bar, context window management, llama.cpp endpoint metadata

## Problem

`src/app_chat_loop.rs` (lines 1165-1177) uses a crude character-based token estimation:

```rust
// Estimate tokens from message content (~4 chars per token)
let mut tokens_in: u64 = 0;
let mut tokens_out: u64 = 0;
for msg in &runtime.messages {
    let est = TerminalUI::estimate_tokens(&msg.content);
    if msg.role == "assistant" {
        tokens_out += est;
    } else {
        tokens_in += est;
    }
}
```

And in `src/ui_terminal.rs`:
```rust
fn estimate_tokens(content: &str) -> u64 {
    (content.len() as u64 + 3) / 4
}
```

This is **wildly inaccurate**:
- Code (Rust, with many short tokens like `{`, `}`, `->`) averages 2-3 chars per token
- English prose averages 3-4 chars per token
- Unicode/non-ASCII text can be 1 char = 1-6 bytes, but tokenization is script-dependent
- The "+3 / 4" formula has no basis in any tokenizer

The codebase already depends on `tiktoken-rs` (Cargo.toml line 77) and has `src/token_counter.rs` (Task 499). But the status bar and context management use the crude estimate instead. For local llama.cpp-compatible endpoints, `tiktoken-rs` may still be only an approximation; Elma should prefer endpoint-provided context length and tokenizer/usage data when available.

Consequences:
1. **Context window overflows**: the status bar shows 4000/8000 tokens, but actual tokens might be 6000/8000. The auto-compaction trigger fires too late.
2. **Budget miscalculations**: `budget_forecaster.rs` (Task 653) may use inaccurate estimates
3. **User confusion**: the token count in the footer is misleading
4. **Violates Rule 5**: grounded answers only — the token count is presented as fact but is fiction

The docs say:
> "Token Counter (Task 499): The token counter (`token_counter.rs`) uses `tiktoken-rs` for accurate token counting... Used for context window management and budgeting."

But `token_counter.rs` is not actually used in the live path.

## Root Cause

The crude estimate was added early as a quick approximation. When `tiktoken-rs` was integrated (Task 499), the old estimate was never replaced in the UI path.

## Proposed Solution

### Phase 1 — Audit all token estimation sites

Find every call to `estimate_tokens()` or inline `(len / 4)` token math:
```bash
grep -rn "estimate_tokens\|len.*4\|chars.*4" src/
```

Expected sites:
- `src/app_chat_loop.rs` (status bar update)
- `src/ui_terminal.rs` (`estimate_tokens` function)
- `src/auto_compact.rs` (compaction trigger)
- `src/budget_forecaster.rs` (budget prediction)
- `src/context_budget.rs` (context budget tracking)

### Phase 2 — Replace with model-aware counting

1. In `src/token_counter.rs`:
   - Ensure `count_tokens(text: &str, model_id: &str, endpoint_caps: &ModelEndpointCaps) -> TokenCount` is exposed
   - Prefer provider-reported usage when available for completed calls
   - Prefer llama.cpp tokenizer or metadata endpoints if available
   - Use `tiktoken-rs` for models where the tokenizer is known to match
   - Use an explicit `Estimator` fallback for unknown models, and mark the status as estimated rather than factual

2. In `src/app_chat_loop.rs`:
   - Replace the manual estimation loop with `token_counter::count_tokens_batch(&runtime.messages, &runtime.model_id, &runtime.endpoint_caps)`
   - Cache the result to avoid re-tokenizing on every status update

3. In `src/ui_terminal.rs`:
   - Delete `estimate_tokens()`
   - Replace with `token_counter::count_tokens()`

4. In `src/auto_compact.rs`:
   - Replace compaction threshold checks with accurate token counts
   - Ensure compaction fires before the context window is exceeded, not after

### Phase 3 — Model-specific tokenizers

`src/model_capabilities.rs` already has `TokenizerKind`:
```rust
pub enum TokenizerKind {
    Tiktoken,
    Cl100kBase,
    Anthropic,
    HuggingFace,
    Estimator,
}
```

1. Map `model_id` and endpoint metadata to `TokenizerKind` in `model_capabilities.rs`
2. Use the correct tokenizer for the active model
3. If the model is unknown, use the best available tokenizer estimate and mark it as estimated in the status/trace metadata

### Phase 3a — Context length detection

1. During provider health initialization, read the llama.cpp-compatible endpoint metadata for context length when available.
2. Store the detected `ctx_max` in runtime state and session metadata.
3. If the endpoint cannot report context length, fall back to configured context length and mark it as configured, not detected.
4. Task 737 should consume this detected context length when deciding whether destructive compaction is necessary.

### Phase 4 — Performance

Tokenization can be expensive. To avoid blocking the UI:

1. Compute token counts asynchronously or cache them
2. Update the status bar token count only when messages change
3. For the status bar, an approximate count updated every N seconds is acceptable if the exact count is computed in the background

## Acceptance Criteria

- [ ] `estimate_tokens()` (char/4) is deleted from the entire codebase
- [ ] All token counts use provider usage, endpoint tokenizer data, `tiktoken-rs`, or an explicit estimated fallback
- [ ] Status bar distinguishes exact, provider-reported, and estimated token counts
- [ ] Detected context length from llama.cpp-compatible endpoints is persisted in session metadata
- [ ] Auto-compaction triggers based on accurate counts
- [ ] Budget forecaster uses accurate counts
- [ ] Performance: status bar update does not cause visible lag
- [ ] `cargo build && cargo test` passes
- [ ] Unit tests verify token counts for sample texts

## Verification Plan

- Unit test: `count_tokens("Hello world", "gpt-4")` returns correct tiktoken count
- Unit test: `count_tokens("fn main() {}", "gpt-4")` returns correct count (not len/4)
- Unit test: unknown model falls back to `cl100k_base`, not char/4
- Integration test: send a long message → status bar updates with accurate count
- Performance test: send 10 messages in quick succession → no UI lag

## Dependencies

- `src/token_counter.rs` (Task 499)
- `src/model_capabilities.rs` (Task 448)
- `src/ui_terminal.rs` (status bar)
- `src/auto_compact.rs` (compaction)
- `src/budget_forecaster.rs` (budgeting)

## Notes

This is a **truthfulness** fix. Presenting `(len / 4)` as a token count is a falsehood. The docs say:

> "Eliminate falsehoods" is priority #1 in runtime behavior priorities.

The crude estimate was acceptable during early prototyping. Now that the system has 600+ tests and enterprise-grade ambitions, accuracy is required.

Do not keep the crude estimate as a "fast path." The correct fast path is caching the tiktoken result, not using a wrong formula.

If `tiktoken-rs` adds too much latency for the status bar, compute tokens when messages are added (event-driven) rather than polling.
