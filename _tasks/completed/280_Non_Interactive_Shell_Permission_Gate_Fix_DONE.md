# Task 280: Fix Non-Interactive Shell Permission Gate Blocking Read-Only Commands

## Problem

When running elma with piped stdin (non-interactive mode), shell commands are blocked even if they are read-only (e.g., `du`, `find`, `ls`).

### Root Cause

In `tool_calling.rs:158`, the hardcoded `is_destructive: true` forces ALL shell tool calls to require permission, and in non-interactive mode the permission gate denies everything:

```rust
// tool_calling.rs - BUG
if !permission_gate::check_permission(args, &command, true, ...).await
//                                                ^^^
//                                                ALWAYS destructive!
```

The `classify_command()` function in `shell_preflight.rs` already exists and correctly identifies read-only commands as `RiskLevel::Safe`, but it's never used.

### Evidence

Trace from failed session:
```
trace: permission_gate: DENIED (non-interactive mode): du -sh sessions/
trace: tool_call: shell DENIED by permission gate
```

This happens even for completely safe commands:
- `du -sh sessions/`
- `find sessions/ -type f -mtime +14 | wc -l`
- `ls -la`

## Solution

Replace hardcoded `true` with actual classification using `classify_command()`:

```rust
// In tool_calling.rs
use crate::shell_preflight::{classify_command, RiskLevel};

let risk = classify_command(&command);
let is_dangerous = matches!(risk, RiskLevel::Dangerous(_));
if !permission_gate::check_permission(args, &command, is_dangerous, tui).await
```

Additionally, in non-interactive mode, allow `RiskLevel::Caution` commands (they can't be confirmed anyway):

```rust
// In permission_gate.rs - allow Caution in non-interactive for read-only ops
if is_non_interactive && !is_dangerous {
    return true;  // Allow non-destructive commands in non-interactive mode
}
```

## Classification Coverage

The existing `classify_command()` handles:

### Safe (~80% of cases)
- `ls`, `cat`, `find`, `du`, `wc`, `grep`, `rg`, `stat`, `file`
- Storage queries: `find -type f | wc -l`
- Git read-only: `git status`, `git log`, `git diff`
- Cargo: `cargo build`, `cargo test`

### Caution (needs confirmation)
- `mv`, `cp` - file movement/copy

### Dangerous (always blocked)
- `rm`, `shred`, `dd`, `> file`, `>> file`

## Implementation Steps

1. **Edit `tool_calling.rs`**: Import `classify_command` and use actual classification
2. **Edit `permission_gate.rs`**: Allow `Safe` and `Caution` commands in non-interactive mode
3. **Build and verify**

## Risks

- False Safe: Some dangerous commands may slip through (mitigated by still blocking Dangerous)
- Pattern-matching not AI-quality (acceptable -Claude Code uses AI classification for this)

## Testing

```bash
echo "if we deleted all sessions older than 2 weeks under sessions how much space will we save?" | cargo run --quiet --
```

Expected: Commands execute in non-interactive mode (only Dangerous commands should be blocked).

## Completion Criteria

- [x] Read-only commands (`du`, `find`, `ls`) work in non-interactive mode
- [x] Build passes
- [x] Non-interactive commands execute without hanging