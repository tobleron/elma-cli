# Task 050: Permission System for Tool Execution

## Priority
**P3 - LOW** (Not critical for now, safety enhancement)

## Problem
Elma can currently execute any command anywhere on the system. While powerful, this could lead to unintended consequences if the model makes mistakes.

## Goal
Add a permission system that:
- Restricts Elma to workspace directory by default
- Prompts user for dangerous operations (delete, system-wide changes)
- Logs all tool executions for audit

## Implementation Notes (Future)

### Reference Implementations
1. **OpenCode** (`~/opencode/internal/permission/`)
   - Permission prompts before tool execution
   - Session-based permission tracking
   - Configurable permission levels

2. **Kolosal** (`~/kolosal-cli/packages/core/src/tools/`)
   - Tool confirmation dialogs
   - Configurable trust levels per tool

### Key Components to Add
1. `src/permission.rs` - Permission service
2. `src/sandbox.rs` - Workspace restriction
3. Permission prompts in TUI
4. Session-based permission cache

### Scope Restrictions (Phase 1)
- All file operations restricted to workspace root
- Shell commands warned if outside workspace
- No restrictions on read-only operations

### Permission Levels (Phase 2)
- **Read**: Always allowed within workspace
- **Write**: Prompt once per session
- **Execute**: Prompt every time
- **Dangerous** (rm -rf, etc.): Explicit confirmation

## Files to Create (Future)
- `src/permission.rs`
- `src/sandbox.rs`

## Files to Modify (Future)
- `src/execution_steps.rs` - Add permission checks
- `src/tools/discovery.rs` - Mark dangerous tools

## Dependencies
- None blocking

## Status
**PARKED** - Not a priority for current development
