# 002 — Restore Protected Path Blocking with Explicit Allowlist

- **Priority**: Critical
- **Category**: Security
- **Depends on**: None
- **Blocks**: 008

## Problem Statement

The `check_protected_paths()` function in `src/shell_preflight.rs:634-639` has been neutered — it returns `None` unconditionally:

```rust
fn check_protected_paths(_command: &str) -> Option<String> {
    // Protected path blocking removed — Elma may operate in other projects
    // where src/, config/, etc. are not Elma-specific directories.
    // Risk classification (classify_command) and dry-run previews still apply.
    None
}
```

Meanwhile, the constants `_PROTECTED_DIRS` and `_PROTECTED_FILES` are prefixed with `_` (dead code suppression) and contain project-specific paths. The documentation in `docs/SECURITY_AND_PERMISSIONS.md` still describes protected paths as blocking, creating a discrepancy between docs and behavior.

This means Elma can execute `rm -rf src/`, `rm Cargo.toml`, `rm -rf sessions/`, or `rm -rf .git/` without any path-based blocking. Only the general risk classification applies (`rm` is "Dangerous"), but in `safe_mode=off`, even Dangerous commands are not blocked (only classified).

## Why This Matters for Small Local LLMs

Small models are more likely to hallucinate or misunderstand file paths. A 4B model might:
- Generate `rm -rf src/` instead of `rm src/old_file.rs`
- Confuse `sessions/` with a user directory
- Delete `.git/` while trying to clean up the working tree
- Write over `Cargo.toml` with a hallucinated version

Without path protection, there is no last line of defense against model mistakes.

## Current Behavior

```
# In safe_mode=off:
$ rm -rf .git/         → classified as "Dangerous" but NOT blocked; executes
$ rm Cargo.toml        → classified as "Dangerous" but NOT blocked; executes  
$ rm -rf sessions/     → classified as "Dangerous" but NOT blocked; executes
```

## Recommended Target Behavior

Restore protected path checking with a principled approach:

1. **Define protected paths as a configuration concept**, not hardcoded project-specific paths
2. **Read protection rules from `.elma-protect`** file in workspace root (or `elma.toml` `[protect]` section)
3. **Provide sensible defaults** that protect common critical paths:
   - `.git/` (VCS history)
   - Any file matching `.elma-protect`
   - Any path in a configurable `protected_paths` list
4. **Distinguish read from write**: Protected paths can always be read but never mutated without explicit confirmation
5. **Three-tier protection**: 
   - `readonly` — can be read, not modified (`.git/`)
   - `caution` — modification requires confirmation (`Cargo.toml`)
   - `open` — no restrictions

## Source Files That Need Modification

- `src/shell_preflight.rs:634-639` — Restore `check_protected_paths()` implementation
- `src/shell_preflight.rs:28-48` — Replace `_PROTECTED_DIRS` and `_PROTECTED_FILES` with new configurable system
- `src/workspace_policy.rs` — Add protected path configuration
- `src/tool_calling.rs` — Add protected path checks to `exec_edit`, `exec_write`, `exec_patch`, `exec_move`, `exec_trash`
- `config/runtime.toml` — Add `[protect]` section
- `elma.toml` — Add `[protect]` section for per-project configuration
- `docs/SECURITY_AND_PERMISSIONS.md` — Update to reflect new behavior

## New Files/Modules

- `src/protected_paths.rs` — Centralized protected path checking, loaded from config
- `_elma-protect` template file for workspace initialization

## Step-by-Step Implementation Plan

1. Create `src/protected_paths.rs` with `ProtectedPaths` struct:
   - `readonly: Vec<PathPattern>` — can read, cannot mutate
   - `caution: Vec<PathPattern>` — mutation requires confirmation
   - Helper: `fn classify(path: &Path, action: PathAction) -> ProtectionLevel`
2. Add `[protect]` section to `config/runtime.toml`:
   ```toml
   [protect]
   readonly = [".git", ".git/**", "target/**"]
   caution = ["Cargo.toml", "Cargo.lock", "*.toml"]
   ```
3. Add `[protect]` section to `elma.toml` for per-project overrides
4. Load protection rules at bootstrap in `app_bootstrap_core.rs`
5. Restore `check_protected_paths()` to use the new system:
   ```rust
   fn check_protected_paths(command: &str, workdir: &Path, protect: &ProtectedPaths) -> Option<String> {
       // Extract paths from command, check against protect rules
   }
   ```
6. Add path checks to mutating tool executors:
   - `exec_edit` — check target path before writing
   - `exec_write` — check target path before creating
   - `exec_patch` — check all affected paths
   - `exec_move` — check source and dest
   - `exec_trash` — check target path
7. Update `docs/SECURITY_AND_PERMISSIONS.md`
8. Add tests for each protection level

## Recommended Crates

- `globset` (already a transitive dependency via `ignore`) — for path pattern matching
- `camino` — for UTF-8 path handling (optional but cleaner)

## Validation/Sanitization Strategy

- Path patterns use glob syntax for consistency with the `glob` tool
- All paths are resolved relative to workspace root before checking
- Canonicalize paths before comparison to prevent `../` escapes
- Symlinks are followed to their target before checking protection level
- Protection checks are applied AFTER preflight classification but BEFORE `permission_gate`

## Testing Plan

1. Unit tests for path pattern matching against various inputs
2. Test that `rm -rf .git/` is blocked even in safe_mode=off
3. Test that `read .git/config` succeeds (read ok on protected paths)
4. Test that `edit Cargo.toml` shows confirmation prompt
5. Test that custom `.elma-protect` patterns work
6. Test that `../` path traversal is caught
7. Test that symlink targets are checked

## Acceptance Criteria

- `rm -rf .git/` is blocked in ALL safe modes
- `read .git/config` works (read-only on protected paths)
- `edit Cargo.toml` requires confirmation
- Protected paths are configurable via `elma.toml`
- All existing shell preflight tests pass
- Documentation matches behavior

## Risks and Migration Notes

- **Migration risk**: Users who rely on Elma operating in arbitrary workspaces may find protection too aggressive. Mitigate with per-project configuration and a `--unsafe-workspace` flag.
- **Performance risk**: Path canonicalization and glob matching on every shell command may add latency. Cache protection results per session.
- The `_PROTECTED_DIRS` and `_PROTECTED_FILES` constants should be REMOVED (not just renamed) once the new system is in place.
