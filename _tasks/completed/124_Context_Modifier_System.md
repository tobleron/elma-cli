# Task 124: Context Modifier System

## Priority
**P2 — Extensibility**
**Created:** 2026-04-05
**Status:** Pending
**Dependencies:** Task 123 (Pre-Tool Hook System)

## Problem

After a destructive command executes (or is blocked), the model's behavior doesn't adapt. Claude Code uses `contextModifier` — tools return functions that change system state mid-conversation. Elma needs similar adaptive behavior.

## Scope

### 1. Context Modifier Interface
```rust
pub(crate) trait ContextModifier {
    fn name(&self) -> &str;
    fn apply(&self, context: &mut ToolContext) -> Result<()>;
}
```

### 2. Built-in Modifiers
- `LowerTrustAfterDestructive` — after destructive command succeeds, require confirmation for next N commands
- `RaiseTrustAfterSafe` — after N consecutive safe commands, relax confirmation requirements
- `LockAfterBudgetExhausted` — after command budget exhausted, lock all mutation tools
- `RememberPathCorrection` — after model uses wrong path (`stress_testing/` vs `_stress_testing/`), inject correction into system prompt

### 3. Integration Points
- `src/tool_calling.rs` — tools can return `Vec<Box<dyn ContextModifier>>`
- `src/context.rs` (new) — `ToolContext` struct with mutable state
- Tool loop applies modifiers between iterations

## Design Principles
- **Adaptive:** Model behavior changes based on demonstrated reliability
- **Transparent:** Trust level changes logged to trace
- **Reversible:** Trust recovers after safe behavior streak
- **Small-model-friendly:** Modifications injected into system prompt, not abstract state

## Verification
1. `cargo build` clean
2. Real CLI: model uses wrong path → path correction injected into next prompt
3. Real CLI: model executes destructive command → trust lowered, next command requires confirmation

## Acceptance Criteria
- [ ] Context modifier trait defined
- [ ] Tools can return modifiers that affect subsequent behavior
- [ ] Built-in modifiers: trust adjustment, path correction, budget lock
- [ ] Modifiers applied between tool loop iterations
- [ ] Trust recovery after safe behavior streak
