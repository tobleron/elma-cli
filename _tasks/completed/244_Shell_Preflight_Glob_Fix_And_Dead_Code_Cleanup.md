# 244: Shell Preflight Glob Match Multi-Star And Dead Function Cleanup

## Status
`pending`

## Priority
Medium — Correctness: `glob_match` silently fails on multi-star patterns; dead `parse_shlex` function.

## Source
Code review findings M-17 and M-18.

**M-18:** `glob_match` in `shell_preflight.rs` only handles a single `*`. Multi-star patterns like `*.test.*` or `backup_*_v*.rs` have `parts.len() > 2` and fall through to `pattern == name`, returning `false`. The dry-run preview for such patterns shows "No files match" — giving the user false confidence before a destructive operation.

**M-17:** `parse_shlex` (line 500) is functionally identical to `try_parse_shlex` but returns `anyhow::Result` instead of `Option`. It's called in `preflight_mv`, `preflight_cp`, `preflight_rm` but the `Err` arm is never propagated — it's immediately matched away identically to `try_parse_shlex`. Dead code.

## Objective
Fix the multi-star glob matcher and remove the dead `parse_shlex` wrapper.

## Scope

### `src/shell_preflight.rs`

**1. Fix `glob_match` for multi-star patterns:**
```rust
fn glob_match(pattern: &str, name: &str) -> bool {
    // Single wildcard shortcut
    if pattern == "*" { return true; }

    // Split on '*' and match each segment sequentially
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.is_empty() { return false; }

    // Name must start with the first segment
    let Some(remaining) = name.strip_prefix(parts[0]) else { return false; };
    let mut pos = remaining;

    // Each intermediate segment must appear in order
    for segment in &parts[1..parts.len()-1] {
        if segment.is_empty() { continue; }
        let Some(idx) = pos.find(segment) else { return false; };
        pos = &pos[idx + segment.len()..];
    }

    // Name must end with the last segment
    if let Some(last) = parts.last() {
        if !last.is_empty() && !pos.ends_with(last) { return false; }
    }
    true
}
```

**2. Remove `parse_shlex` (lines 500–502) and update call sites:**
Replace every `parse_shlex(args)?` call with:
```rust
try_parse_shlex(args).ok_or_else(|| anyhow::anyhow!("invalid shell quoting in arguments"))?
```
Or inline as:
```rust
let parts = match try_parse_shlex(args) {
    Some(p) => p,
    None => return PreflightResult {
        risk: RiskLevel::Caution,
        error_guidance: Some("invalid shell quoting in arguments".to_string()),
        dry_run_preview: None,
    },
};
```

**3. Add tests for multi-star glob:**
```rust
#[test]
fn test_glob_match_multi_star() {
    assert!(glob_match("*.test.*", "foo.test.rs"));
    assert!(glob_match("backup_*_v*.rs", "backup_main_v2.rs"));
    assert!(!glob_match("*.test.*", "foo.rs"));
    assert!(glob_match("*", "anything"));
    assert!(glob_match("exact", "exact"));
    assert!(!glob_match("exact", "inexact"));
}
```

## Verification
- `cargo build` passes.
- `cargo test shell_preflight` passes — including new multi-star tests.
- `rg 'parse_shlex\b' src/shell_preflight.rs` returns zero matches (only `try_parse_shlex` remains).
- Dry-run preview for `rm *.test.rs` shows correct matched files.

## References
- `src/shell_preflight.rs:500–519` (parse_shlex, glob_match)
- `src/shell_preflight.rs:693–791` (preflight_mv, preflight_cp, preflight_rm callers)
