# 238: Permission Gate Prefix-Match Bypass Fix

## Status
`completed`

## Priority
P0 — Security: approval bypass allows model to escalate approved commands.

## Source
Code review finding C-3. `ApprovalCache::is_approved` uses `command.starts_with(pattern)` for prefix matching. This means approving `rm /tmp/test` also silently approves `rm /tmp/test_important_file`. A model could craft a command that starts with an approved prefix to bypass the permission gate.

## Objective
Tighten the prefix-match logic in `ApprovalCache::is_approved` to require a proper word boundary (space, `/`, or end-of-string) after the approved prefix. Apply the same fix to mutex poisoning throughout the file.

## Scope

### `src/permission_gate.rs`

**1. Fix `is_approved` word-boundary check:**
```rust
fn is_approved(&self, command: &str) -> bool {
    // Exact match always wins
    if self.approved.contains(command) {
        return true;
    }
    // Prefix match: only allow if followed by a word boundary
    for pattern in &self.approved {
        if command.starts_with(pattern.as_str()) {
            let rest = &command[pattern.len()..];
            // Next char must be space, '/', or end-of-string
            if rest.is_empty() || rest.starts_with(' ') || rest.starts_with('/') {
                return true;
            }
        }
    }
    false
}
```

**2. Fix mutex poisoning: replace `lock().ok()`:**

In `check_permission`, replace the two separate `.lock().ok()` calls with a single lock acquisition:
```rust
let (already_approved, is_non_interactive) = {
    let cache = approval_cache().lock().unwrap_or_else(|e| e.into_inner());
    (cache.is_approved(command), cache.non_interactive)
};
if already_approved { return true; }
if is_non_interactive { ... }
```

Apply `unwrap_or_else(|e| e.into_inner())` to all `approval_cache().lock()` calls in the file.

**3. Update tests** to cover the boundary behavior:
```rust
#[test]
fn test_approval_prefix_does_not_bypass_boundary() {
    let mut cache = ApprovalCache::new(false);
    cache.approve("rm /tmp/test");
    assert!( cache.is_approved("rm /tmp/test"));       // exact match
    assert!(!cache.is_approved("rm /tmp/test_other")); // no boundary → denied
    assert!( cache.is_approved("rm /tmp/test file2")); // space boundary → ok
}
```

## Verification
- `cargo build` passes.
- `cargo test permission_gate` passes.
- New boundary test passes.
- Existing tests `test_approval_cache_exact_match`, `test_approval_cache_prefix_match` still pass (the prefix test approves `"mv *.sh "` which ends in a space — boundary is preserved).

## References
- `src/permission_gate.rs:32–43` (is_approved)
- `src/permission_gate.rs:82–107` (check_permission double-lock)
