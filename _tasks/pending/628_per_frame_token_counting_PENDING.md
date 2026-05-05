# 628 Replace Per-Frame BPE Token Counting

## Summary
`token_counter::count_tokens` re-encodes the entire streaming text with tiktoken-rs on every draw frame. During long streaming responses (~5000+ chars), this re-tokenizes at 25fps.

## Affected Files
- `src/ui/ui_terminal.rs:901` — `draw_claude` calls `count_tokens` on streaming.thinking and streaming.content

## Current Behavior
- Every draw frame: `count_tokens(&thinking)` + `count_tokens(&content)`
- tiktoken-rs BPE encoding is O(n) in text length
- During a 30s streaming response, this runs ~750 times on progressively larger text (re-encodes from scratch each time)

## Proposed Fix
- Use `chars / 4` approximation for the live footer display (accurate enough for visual indicator)
- Real BPE counting at turn boundaries only (when streaming finishes)
- Alternatively: cache token count with text length, recompute only when length changes (incrementally during streaming)
- Add `token_estimate: u64` field to streaming state, updated per-delta with `delta.chars().count() / 4`

## Estimated CPU Savings
~5% of per-frame render CPU, but eliminates a per-frame sys-call-like cost

## Status
PENDING
