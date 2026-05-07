# Task 759: Deferred Replace Implicit Global Re-Exports In main.rs With Explicit Imports

## Deferred Status

Deferred during pending queue review on 2026-05-07.

Reason: this is a broad mechanical import refactor across most of the crate. It may improve long-term maintainability, but it does not directly address the current reliability, context, instruction-following, or debugging gaps. Running it now would create a large conflict surface and could obscure behavioral regressions from the higher-priority agent stability tasks.

## Type

Architecture / Maintainability / Code Quality

## Severity

Medium

## Scope

Module system, imports, compilation

## Problem

`src/main.rs` contains **335 `pub(crate) use` re-exports** that make every type available as `crate::TypeName` without explicit imports:

```rust
pub(crate) use abstractions::*;
pub(crate) use agent_fsm::*;
pub(crate) use atomic_write::*;
// ... 332 more ...
pub(crate) use workspace::*;
pub(crate) use workspace_policy::*;
```

This design:
1. **Hides dependencies**: a module can use `crate::Foo` without declaring where `Foo` comes from
2. **Increases compile time**: the compiler must resolve all re-exports for every module
3. **Makes refactoring harder**: moving a type between modules requires updating `main.rs` re-exports
4. **Prevents dead code detection**: unused types may appear "used" because they're re-exported
5. **Violates Rust conventions**: `pub(crate) use` should be for shared types, not every module

The DEVELOPMENT.md says:

> "`main.rs` declares 167 modules and re-exports 50+ of them as `pub(crate) use`. This means most types are available as `crate::TypeName` without explicit imports in sub-modules."

But the actual count is **335 re-exports**, not 50+. The docs are undercounting by 6x.

## Root Cause

The re-export pattern was introduced to reduce boilerplate imports. It scaled poorly as the codebase grew from 50 modules to 267.

## Proposed Solution

### Phase 1 — Audit critical re-exports

Identify which re-exports are actually used as `crate::TypeName` vs. imported explicitly. Use:
```bash
grep -rn "crate::[A-Z]" src/ | sed 's/.*crate::\([A-Z][A-Za-z0-9_]*\).*/\1/' | sort | uniq -c | sort -rn | head -50
```

### Phase 2 — Keep essential re-exports only

Keep `pub(crate) use` only for:
- Core types used in > 50% of modules (`ChatMessage`, `Step`, `AppRuntime`, `Args`)
- Error types (`Result`, `Error`)
- Very common utilities (`trace`, `trace_verbose`)

Expected keep list: ~ 20 re-exports, not 335.

### Phase 3 — Add explicit imports

In every module that uses a removed re-export:
1. Add explicit `use crate::module::Type;`
2. Or use `super::Type` if in a submodule
3. Or use `use crate::Type` if the type is genuinely shared

### Phase 4 — Delete re-exports

Remove all `pub(crate) use` lines except the essential ~20.

### Phase 5 — Enforce with lint

Add a CI check or clippy lint that fails if `main.rs` has > 25 `pub(crate) use` lines.

## Acceptance Criteria

- [ ] `main.rs` has ≤ 25 `pub(crate) use` re-exports
- [ ] All modules compile with explicit imports
- [ ] `cargo build && cargo test` passes
- [ ] `cargo fmt` passes
- [ ] No regression in functionality

## Verification Plan

- `grep -c "pub(crate) use" src/main.rs` → ≤ 25
- `cargo build` → success
- `cargo test` → success
- `cargo clippy` → no new warnings

## Dependencies

- `src/main.rs` (all re-exports)
- Every module in `src/` (may need import additions)

## Notes

This is a tedious but mechanical refactor. Use `sed` or a Rust tool to auto-generate import additions.

The goal is not to eliminate all `crate::` references — some are appropriate. The goal is to make dependencies explicit so developers can see what a module actually uses.

Do not change module visibility (`pub(crate) mod` → `mod`). Keep modules public within the crate; just require explicit imports.
