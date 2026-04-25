# Task 266: Automatic Large Result Persistence Mechanism

## Status: DONE
## Priority: MEDIUM

## Problem Statement
Elma lacks automatic persistence of large tool outputs, leading to potential context window flooding. Claude-Code's FileReadTool automatically saves outputs exceeding maxResultSizeChars to temporary files and returns paths instead.

## Analysis from Claude-Code
- Tools have `maxResultSizeChars` property
- Outputs exceeding limit automatically saved to temp files
- Model receives file path for access instead of full content
- Prevents context window overflow from large outputs

## Solution Architecture
1. **Result Size Tracking**: Add size limits to tool definitions
2. **Automatic Persistence**: Create `src/tool_result_storage.rs` for large output handling
3. **Path Provision**: Return file paths for oversized results
4. **Integration**: Wire into streaming_tool_executor.rs

## Implementation Steps
1. Add maxResultSizeChars to tool configuration
2. Create ToolResultStorage for file persistence
3. Implement size checking in tool execution
4. Automatically persist oversized results to `_dev-system/tmp`
5. Return file paths instead of content
6. Add cleanup mechanisms for temp files

## Integration Points
- `src/streaming_tool_executor.rs`: Add size checking and persistence
- `src/tool_result_storage.rs`: New module for result management
- `src/tools.rs`: Update tool definitions with size limits
- `src/orchestration_loop.rs`: Handle file path responses
- `src/execution_steps_*.rs`: Set appropriate size limits

## Success Criteria
- Large outputs automatically saved to temp files
- Context window protected from overflow
- File paths provided for oversized results
- Cleanup of temporary files works
- `cargo build` passes

## Files to Create/Modify
- `src/tool_result_storage.rs` (new)
- `src/streaming_tool_executor.rs` (modify)
- `src/tools.rs` (modify)
- `src/orchestration_loop.rs` (modify)
- Tool definition files (modify)

## Risk Assessment
- LOW: Additive feature, doesn't break existing flows
- Temp file management needs careful implementation
- Can be disabled if issues arise
- Backward compatible