# 581 — Remove Dead Protected Path Constants

- **Priority**: Low
- **Category**: Refactoring
- **Depends on**: 551 (restore protected path blocking)
- **Blocks**: None

## Problem Statement

`shell_preflight.rs:28-48` defines two constants with leading underscores (Rust convention for "intentionally unused"):

```rust
const _PROTECTED_DIRS: &[&str] = &[
    "sessions/", "config/", "_tasks/", "_dev-tasks/",
    "src/", ".git/", "target/", "_claude_code_src/",
];

const _PROTECTED_FILES: &[&str] = &[
    "Cargo.toml", "Cargo.lock", "rust-toolchain.toml",
    ".gitignore", "AGENTS.md", "QWEN.md",
];
```

These were part of the original protected path system that was disabled (Task 002 / 551). They are dead code that:
1. Confuses readers about whether path protection is active
2. References project-specific paths that don't apply to other workspaces
3. Includes a reference to `QWEN.md` which may not exist

After Task 551 restores protected path blocking with a configurable system, these hardcoded constants should be removed entirely.

## Source Files That Need Modification

- `src/shell_preflight.rs:28-48` — Remove `_PROTECTED_DIRS` and `_PROTECTED_FILES`

## Acceptance Criteria

- Dead constants removed from `shell_preflight.rs`
- No compilation warnings
- No references to removed constants elsewhere in the codebase

## Risks and Migration Notes

- Must be done AFTER Task 551 to avoid removing constants before the new system is in place.
- Check for any test references to these constants (there are tests that reference them in the test module).
