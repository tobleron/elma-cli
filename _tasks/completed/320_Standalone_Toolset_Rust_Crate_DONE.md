# Task 320: Standalone Toolset Rust Crate (Proposal 011)

**Status:** completed  
**Proposal:** [docs/_proposals/011-fixed-toolset-rust-crate.md](../../docs/_proposals/011-fixed-toolset-rust-crate.md)  
**Depends on:** Task 317 (check_fn) — benefit is larger with check_fn in the crate  

## Summary

Extract `DynamicToolRegistry`, `ToolDefinitionExt`, and tool definitions from `src/tool_registry.rs` into a standalone `elma-tools` crate with per-tool modules.

## Why

The tool registry is currently a single 200+ line function. Extracting into a standalone crate with per-tool modules enables independent testing, better separation of concerns, future sharing (e.g. server mode), and follows the Hermes Agent pattern of tools as a first-class subsystem.

## Implementation Steps

1. Create `elma-tools/` directory with `Cargo.toml` and `src/`
2. Move `ToolDefinition`, `ToolFunction` types to `elma-tools/src/types.rs`
3. Move `ToolDefinitionExt`, `DynamicToolRegistry` to `elma-tools/src/registry.rs`
4. Split `register_default_tools()` into `elma-tools/src/tools/` modules (one per tool)
5. Add `elma-tools` dependency to root `Cargo.toml`
6. Update all imports in main crate
7. Re-export `ToolDefinition` from `types_api.rs` for backward compat
8. Build and test both crates

## Success Criteria

- [x] `elma-tools/` crate exists with own `Cargo.toml`
- [x] Per-tool modules under `elma-tools/src/tools/`
- [x] `DynamicToolRegistry::new()` uses `register_all()` builder pattern
- [x] Root crate compiles with `use elma_tools::*` imports
- [x] `cargo test -p elma-tools` passes
- [x] `cargo build` (full workspace) succeeds
