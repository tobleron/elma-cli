# 577 — Add ANSI Escape Sequence Sanitization Boundary

- **Priority**: Medium
- **Category**: Sanitization
- **Depends on**: None
- **Blocks**: None

## Problem Statement

ANSI escape sequences in shell output can leak into:
1. **Model context**: Tool results injected into conversation — ANSI codes consume tokens and confuse small models
2. **Terminal rendering**: Malformed ANSI sequences can corrupt the TUI
3. **Evidence ledger**: Raw evidence stored with ANSI codes
4. **Session persistence**: ANSI codes stored in session.json

The `evidence_ledger.rs` `add_entry()` method strips ANSI codes (line 177-180), but other paths may not:
- Direct tool result injection into messages
- Session flush to transcript files
- Tool result storage

The `strip-ansi-escapes` crate is listed as a dependency (Cargo.toml:78) but may not be used uniformly.

## Why This Matters for Small Local LLMs

Small models have limited context windows. ANSI escape sequences can consume 10-50% of a tool result's token budget without adding information. Additionally, small models may interpret ANSI codes as formatting instructions and produce garbled output.

## Current Behavior

ANSI stripping exists in `evidence_ledger.rs` but may be missing from:
- `tool_loop.rs` — When tool results are injected as ChatMessage
- `tool_result_storage.rs` — When tool results are persisted
- `session_flush.rs` — When tool results are written to transcript

## Recommended Target Behavior

Create a single ANSI sanitization boundary that ALL tool output passes through:

```rust
// src/sanitize.rs
pub fn sanitize_tool_output(raw: &str) -> String {
    // 1. Strip ANSI escape sequences
    // 2. Strip terminal control characters
    // 3. Normalize whitespace (optional, configurable)
    // 4. Truncate null bytes
}
```

Apply at the right boundary:
- After tool execution, before the result enters any other system
- In `execute_tool_call()` return value
- OR in the tool lifecycle (Task 560)

## Source Files That Need Modification

- `src/tool_calling.rs` — Apply sanitization to all tool results
- `src/evidence_ledger.rs` — Remove redundant stripping (done at boundary)
- `src/tool_result_storage.rs` — Remove redundant stripping
- `src/session_flush.rs` — Remove redundant stripping

## New Files/Modules

- `src/sanitize.rs` — Centralized output sanitization

## Acceptance Criteria

- All tool output passes through single sanitization function
- ANSI codes never appear in model context
- ANSI codes never appear in evidence ledger or session storage
- Terminal control characters are stripped
- Existing ANSI stripping calls are consolidated to the new boundary

## Risks and Migration Notes

- Over-stripping could remove intentional formatting from tool output (e.g., color-coded grep results). Consider preserving some ANSI codes in terminal transcript while stripping for model context.
