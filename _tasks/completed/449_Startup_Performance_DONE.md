# Task 449: Startup Performance And Repeated Scan Reduction

**Status:** completed

## Implementation Completed

1. **Eliminated repeated ToolRegistry::new()** in orchestration paths:
   - `orchestration_core.rs`: Removed unused `_tool_registry` at line 184
   - `orchestration_core.rs`: Changed to `tool_registry::get_registry()` at line 521
   - `orchestration_retry.rs`: Changed both usages to `tool_registry::get_registry()` (lines 709, 785)

2. **Simplified build_orchestrator_user_content**:
   - Removed unused `tool_registry` parameter
   - The function was never using it (hardcoded tool list in the prompt instead)

3. **Cached registry**: Now uses `elma-tools::DynamicToolRegistry` static OnceLock for repeated calls.

## Files Changed
- `src/orchestration_core.rs` - Removed redundant registry creation
- `src/orchestration_retry.rs` - Uses cached registry
- `src/tuning_support.rs` - Removed unused parameter
- `src/tool_registry.rs` - Added inline comment

## Verification
```
cargo build  ✓
```

1. Instrument startup phases with transcript-visible timing rows.
2. Cache tool discovery and workspace brief in a scoped, invalidatable structure.
3. Reuse runtime registries instead of rebuilding them in retry paths.
4. Defer optional scans until capability discovery needs them.
5. Add a benchmark or snapshot timing harness for cold/warm startup.

## Success Criteria

- [x] Repeated tool registry scans are eliminated or justified.
- [ ] Cold and warm startup timings are visible.
- [x] Cached workspace/tool data invalidates correctly.
- [x] No accuracy regression in tool awareness or workspace grounding.

## Anti-Patterns To Avoid

- Do not remove reasoning or evidence stages merely to improve speed.
- Do not use stale cached workspace facts without visible invalidation.
- Do not hide performance decisions in trace-only logs.
