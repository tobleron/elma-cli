# Task 264: Dynamic Tool Registry With Searchable Capabilities

## Status: PENDING
## Priority: HIGH

## Problem Statement
Elma currently includes all tool schemas in system prompts, consuming tokens and causing confusion. Claude-Code's ToolSearchTool dynamically discovers tools with capability hints, allowing the model to find and load necessary tools without bloating prompts.

## Analysis from Claude-Code
- Tools have "deferred" tool names and search hints (3-10 word capability phrases)
- Model can use ToolSearchTool to find/load tools dynamically
- Reduces prompt token usage and confusion from too many schemas

## Solution Architecture
1. **Tool Registry**: Create `src/tool_registry.rs` with searchable tool database
2. **Capability Hints**: Add `search_hint` field to tool definitions (e.g., "read file contents", "execute shell commands")
3. **Dynamic Loading**: Implement `ToolSearchTool` that searches registry by capability
4. **Integration**: Wire into existing tool_discovery.rs system

## Implementation Steps
1. Define ToolCapability struct with hint strings
2. Create ToolRegistry with search functionality
3. Update tool definitions to include capability hints
4. Implement ToolSearchTool as new tool type
5. Integrate with orchestration loop
6. Test with existing tool set

## Integration Points
- `src/tool_discovery.rs`: Extend existing discovery
- `src/tools.rs`: Update tool loading
- `src/orchestration_loop.rs`: Add dynamic tool injection
- `config/defaults/`: Update intel unit prompts to reference search hints

## Success Criteria
- Reduced token usage in system prompts
- Dynamic tool discovery working in real CLI
- No regression in existing tool functionality
- `cargo build` passes
- Behavioral probes pass

## Files to Create/Modify
- `src/tool_registry.rs` (new)
- `src/tools.rs` (modify)
- `src/tool_discovery.rs` (modify)
- Tool definition files (modify)
- Config files (modify)

## Risk Assessment
- LOW: Backward compatible, additive feature
- Can be rolled back if issues arise
- Doesn't change existing orchestration flow