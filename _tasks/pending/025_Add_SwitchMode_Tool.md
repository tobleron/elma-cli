# 140 Add SwitchMode Tool

## Continuation Checklist
- [ ] Re-read this task and all linked source/task references before editing.
- [ ] Confirm the task is still valid against current `_tasks/TASKS.md`, `AGENTS.md`, and active master plans.
- [ ] Move or keep this task in `_tasks/active/` before implementation work begins.
- [ ] Inspect the current code/config/docs touched by this task and note any drift from the written plan.
- [ ] Implement the smallest coherent change set that satisfies the next unchecked item.
- [ ] Add or update focused tests, probes, fixtures, or snapshots for the changed behavior.
- [ ] Run `cargo fmt --check` and fix formatting issues.
- [ ] Run `cargo build` and resolve all build errors or warnings introduced by this task.
- [ ] Run targeted `cargo test` commands and any task-specific probes listed below.
- [ ] Run real CLI or pseudo-terminal verification for any user-facing behavior.
- [ ] Record completed work, verification output, and remaining gaps in this task before stopping.
- [ ] Ask for sign-off before moving this task to `_tasks/completed/`.

## Summary
Add tool for switching modes during autonomous execution.

## Reference
- Roo-Code: `~/Roo-Code/src/core/tools/SwitchModeTool.ts`

## Implementation

### 1. Define SwitchMode Tool
File: `src/tools/switch_mode.rs` (new)
- Tool name: `switch_mode`
- Parameters:
  - `mode`: target mode name

### 2. Integrate with Tool Registry
File: `src/tool_calling.rs`
- Add `SwitchMode` to tool definitions

### 3. Handle Mode Switch
File: `src/mode_manager.rs`
- Add `switch_from_tool(mode: Mode)` method
- Validate mode exists before switching

## Tool Definition JSON
```json
{
  "name": "switch_mode",
  "description": "Switch to a different mode during execution",
  "parameters": {
    "mode": {"type": "string", "enum": ["architect", "code", "ask", "debug", "orchestrator"]}
  }
}
```

## Verification
- [ ] `cargo build` passes
- [ ] Tool appears in tool list
- [ ] Mode switches correctly from tool call