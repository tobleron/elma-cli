# Task 272: Safe Mode Toggle System For Permission Levels

## Status: PENDING
## Priority: MEDIUM

## Problem Statement
Elma lacks a simple safe/unsafe mode toggle. Open-Interpreter's safe_mode (ask/on/off) provides clear permission levels for shell command execution.

## Analysis from Open-Interpreter
- `safe_mode` with ask/on/off settings
- Clear permission boundaries
- Prevents unauthorized code execution
- Simple user control over safety

## Solution Architecture
1. **Mode Configuration**: Add safe_mode to configuration system
2. **Permission Integration**: Wire into permission_gate.rs
3. **Shell Command Control**: Modify shell execution based on mode
4. **User Feedback**: Clear indication of current mode

## Implementation Steps
1. Add safe_mode configuration (ask/on/off)
2. Update permission_gate.rs with mode checking
3. Modify shell execution to respect mode
4. Add mode status display in UI
5. Implement mode switching commands
6. Test permission boundaries

## Integration Points
- Configuration system (TOML files)
- `src/permission_gate.rs`: Core permission logic
- `src/execution_steps_shell.rs`: Shell command execution
- UI components for mode display
- Command parsing for mode switching

## Success Criteria
- Clear safe/unsafe mode boundaries
- Shell commands respect permission mode
- User can easily switch modes
- No security bypasses possible
- `cargo build` passes

## Files to Create/Modify
- Configuration files (modify)
- `src/permission_gate.rs` (modify)
- `src/execution_steps_shell.rs` (modify)
- UI mode display components (new)
- Mode switching command handling (new)

## Risk Assessment
- MEDIUM: Security-critical feature
- Need thorough testing of permission boundaries
- Backward compatible (defaults to safe)
- Can be implemented incrementally