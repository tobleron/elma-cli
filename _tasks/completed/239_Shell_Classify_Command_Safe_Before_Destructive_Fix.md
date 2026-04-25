# 239: Shell Classify Command — Fix Safe-Before-Destructive Ordering

## Status
`completed`

## Priority
P0 — Security: pipe-safe patterns short-circuit before pipe-destructive check.

## Source
Code review finding C-4. In `shell_preflight.rs`, `classify_command` checks `PIPE_SAFE_READ_ONLY` (line 192) before `PIPE_DESTRUCTIVE_PATTERNS` (line 198). A command containing both a safe and destructive pipe (e.g. `cat f | xargs stat | xargs rm`) hits the safe pattern first and returns `RiskLevel::Safe`, bypassing the dangerous classification entirely.

## Objective
Reorder the pattern checks so destructive patterns always win over safe ones. Add regression tests for mixed-pipe commands.

## Scope

### `src/shell_preflight.rs`

**1. Reorder checks in `classify_command`:**
Move the `PIPE_DESTRUCTIVE_PATTERNS` loop to execute before the `PIPE_SAFE_READ_ONLY` loop:

```rust
pub(crate) fn classify_command(command: &str) -> RiskLevel {
    let cmd = command.trim();
    if cmd.is_empty() { return RiskLevel::Safe; }

    // DESTRUCTIVE must be evaluated before safe overrides
    for (pattern, reason) in PIPE_DESTRUCTIVE_PATTERNS {
        if cmd.contains(pattern) {
            return RiskLevel::Dangerous(
                format!("BULK DESTRUCTIVE: {} pattern detected.", reason)
            );
        }
    }

    // While-read loops
    if cmd.contains("| while read") || cmd.contains("|while read") {
        for keyword in WHILE_LOOP_DESTRUCTIVE_KEYWORDS {
            if cmd.contains(keyword) {
                return RiskLevel::Dangerous(
                    "BULK DESTRUCTIVE: while-read loop contains destructive operation.".to_string()
                );
            }
        }
        return RiskLevel::Safe;
    }

    // Safe read-only pipes (only relevant when no destructive pipe found above)
    for pattern in PIPE_SAFE_READ_ONLY {
        if cmd.contains(pattern) { return RiskLevel::Safe; }
    }

    // ... rest unchanged
}
```

**2. Add regression tests:**
```rust
#[test]
fn test_mixed_pipe_destructive_wins() {
    // Even though "| xargs stat" is in PIPE_SAFE_READ_ONLY, the later "| xargs rm" must win
    let cmd = "find . | xargs stat | xargs rm";
    assert!(matches!(classify_command(cmd), RiskLevel::Dangerous(_)));
}

#[test]
fn test_safe_pipe_alone_is_safe() {
    let cmd = "find . -name '*.rs' | xargs stat";
    assert!(matches!(classify_command(cmd), RiskLevel::Safe));
}

#[test]
fn test_destructive_pipe_alone_is_dangerous() {
    let cmd = "find . | xargs rm -rf";
    assert!(matches!(classify_command(cmd), RiskLevel::Dangerous(_)));
}
```

**3. Also harden `EXECUTED_COMMANDS` mutex poisoning** in `execution_steps_shell_exec.rs`:
- In `is_duplicate_command` and `clear_executed_commands_cache`, replace `.lock().ok()` / `.lock().unwrap()` with:
  ```rust
  let mut cache = EXECUTED_COMMANDS.lock().unwrap_or_else(|e| e.into_inner());
  ```

## Verification
- `cargo build` passes.
- `cargo test shell_preflight` passes — all existing and new tests.
- `cargo test execution_steps_shell_exec` passes.
- Manual: run Elma and attempt `find . | xargs stat | xargs rm` — should be blocked as Dangerous.

## References
- `src/shell_preflight.rs:185–273` (classify_command)
- `src/shell_preflight.rs:192–198` (safe vs destructive ordering)
- `src/execution_steps_shell_exec.rs:109–125` (mutex poisoning)
