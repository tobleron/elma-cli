# 578 — Consolidate Duplicate normalize_shell_signal Implementations

- **Priority**: Medium
- **Category**: Refactoring
- **Depends on**: None
- **Blocks**: None

## Problem Statement

`normalize_shell_signal()` is implemented in TWO places with subtly different logic:

1. **`tool_loop.rs:701-718`**: Collapses ALL digits to `#`, then replaces `s_#_#` with `s_SESSION`
2. **`stop_policy.rs:720-754`**: Collapses only 4+ digit numbers to `#`, then replaces `s_SESSION` (line 753 is a no-op replacement)

The `tool_loop.rs` version is more aggressive (collapses all digits), while the `stop_policy.rs` version is more conservative (only 4+ digit numbers). This means:

1. `tool_loop.rs` uses `normalize_shell_signal()` for stagnation signal detection
2. `stop_policy.rs` uses its own `normalize_shell_signal()` for repeated-command detection

These could produce different normalized signals for the same command, leading to inconsistent stagnation detection.

Both also reference `s_SESSION` but `tool_loop.rs` replaces `s_#_#` with `s_SESSION` while `stop_policy.rs` does `out.replace("s_SESSION", "s_SESSION")` (a no-op).

## Why This Matters for Small Local LLMs

Stagnation detection is critical for small models — they're more prone to repeating commands. If the two implementations diverge, the system may fail to detect stagnation or trigger false positives.

## Current Behavior

```rust
// tool_loop.rs — collapses ALL digits
fn normalize_shell_signal(cmd: &str) -> String {
    for ch in cmd.chars() {
        if ch.is_ascii_digit() {
            out.push('#');
            continue;
        }
        out.push(ch);
    }
    out.replace("s_#_#", "s_SESSION")  // replaces session IDs
}

// stop_policy.rs — collapses 4+ digit numbers
pub(crate) fn normalize_shell_signal(cmd: &str) -> String {
    if current_number.len() >= 4 { out.push('#'); }
    else { out.push_str(&current_number); }
    out.replace("s_SESSION", "s_SESSION") // NO-OP
}
```

## Recommended Target Behavior

Create ONE canonical `normalize_shell_signal()` in a shared module:

```rust
/// Normalize a shell command string for repeated-command detection.
/// Preserves small numbers (limits, offsets) while collapsing large numbers
/// (timestamps, IDs, session identifiers) and session paths.
pub fn normalize_shell_signal(cmd: &str) -> String {
    // 1. Collapse numbers with 4+ digits to '#' (timestamps, IDs)
    // 2. Replace session path patterns (e.g., s_001_abc123) with "s_SESSION"
    // 3. Preserve small numbers (<4 digits) used in limits/offsets
}
```

## Source Files That Need Modification

- `src/tool_loop.rs` — Remove duplicate, import from shared location
- `src/stop_policy.rs` — Remove duplicate, import from shared location

## New Files/Modules

- Move to `src/text_utils.rs` (already exists with related text utilities)

## Acceptance Criteria

- Single `normalize_shell_signal()` function used everywhere
- Both callers produce identical normalized signals
- Preserves small numbers (<4 digits) in command arguments
- Collapses large numbers (4+ digits) and session paths

## Risks and Migration Notes

- Low risk — the functions already exist; this just consolidates them.
- Determine which behavior is correct: conservative (4+ digits only, from stop_policy) or aggressive (all digits, from tool_loop). The conservative approach is preferred to avoid over-normalization (e.g., `head -20` should remain distinct from `head -10`).
