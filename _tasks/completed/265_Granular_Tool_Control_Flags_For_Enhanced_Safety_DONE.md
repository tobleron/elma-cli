# Task 265: Granular Tool Control Flags For Enhanced Safety

## Status: DONE
## Priority: HIGH

## Problem Statement
Elma's Step enum lacks granular control flags like isReadOnly, isDestructive, isConcurrencySafe, and interruptBehavior. Claude-Code uses these flags to provide orchestration with critical information for permissions and execution safety.

## Analysis from Claude-Code
- `isReadOnly`: Tool doesn't modify workspace
- `isDestructive`: Tool can delete/modify files - requires user confirmation
- `isConcurrencySafe`: Tool can run concurrently with others
- `interruptBehavior`: How tool handles interruption (cancel/graceful/complete)

## Solution Architecture
1. **StepCommon Extension**: Add control flags to StepCommon struct
2. **Permission Integration**: Wire isDestructive into permission_gate.rs
3. **Concurrency Control**: Use isConcurrencySafe in orchestration_loop.rs
4. **Interrupt Handling**: Implement interruptBehavior in streaming_tool_executor.rs

## Implementation Steps
1. Extend StepCommon with control flags (defaults provided)
2. Update all Step variants to use extended StepCommon
3. Modify permission_gate.rs to check isDestructive flag
4. Update orchestration loop to respect isConcurrencySafe
5. Implement interrupt behavior handling
6. Add flag validation in step creation

## Integration Points
- `src/types_core.rs`: Extend StepCommon struct
- `src/permission_gate.rs`: Check isDestructive flag
- `src/orchestration_loop.rs`: Respect concurrency flags
- `src/streaming_tool_executor.rs`: Handle interrupt behavior
- `src/execution_steps_*.rs`: Set appropriate flags per step type

## Success Criteria
- Destructive operations require explicit confirmation
- Concurrent execution respects safety flags
- Interrupt handling works correctly
- No breaking changes to existing step usage
- `cargo build` passes

## Files to Create/Modify
- `src/types_core.rs` (modify StepCommon)
- `src/permission_gate.rs` (modify)
- `src/orchestration_loop.rs` (modify)
- `src/streaming_tool_executor.rs` (modify)
- All `execution_steps_*.rs` files (modify)

## Risk Assessment
- MEDIUM: Changes Step enum structure
- Need careful testing of permission flows
- Backward compatible with defaults
- Can be incrementally rolled out