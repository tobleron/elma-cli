# Task 270: Model Context Protocol Integration For Dynamic Capabilities

## Backlog Reconciliation (2026-05-02)

Resume only after Task 490 defines the current MCP extension gateway and Task 447 defines tool-context budgeting. Treat this task as historical architecture input, not an implementation plan.


## Status: DEFERRED

**Reason**: Elma achieves similar extensibility via intel units and skills. MCP is a viable alternative that shows industry success (97M downloads, 28% Fortune 500 adoption), but adds unnecessary attack surface for Elma's small-model-first mission.

**Awareness**: MCP is aware and documented. Revisit if:
- Intel units prove insufficient for ecosystem integration
- MCP server ecosystem matures with more reliable servers
- Enterprise features require standardized protocol
## Priority: LOW

## Problem Statement
Elma lacks Model Context Protocol (MCP) integration. Goose and Crush use MCP for dynamic capability discovery and extensibility while maintaining performance.

## Analysis from Goose/Crush
- MCP enables dynamic capability discovery
- Agents discover capabilities from registered MCP servers
- Highly extensible architecture
- Better ecosystem integration

## Solution Architecture
1. **MCP Client**: Create `src/mcp_client.rs` for protocol handling
2. **Server Registry**: Implement MCP server discovery and registration
3. **Capability Mapping**: Map MCP capabilities to Elma tools/steps
4. **Dynamic Loading**: Runtime capability discovery and loading

## Implementation Steps
1. Implement MCP protocol client
2. Create server registration system
3. Map MCP capabilities to Elma operations
4. Integrate with existing tool system
5. Add configuration for MCP servers
6. Test with sample MCP servers

## Integration Points
- `src/mcp_client.rs`: New MCP protocol implementation
- `src/tool_discovery.rs`: Extend with MCP discovery
- `src/tools.rs`: Dynamic tool loading from MCP
- Configuration system for MCP servers
- Existing orchestration loop (minimal changes)

## Success Criteria
- MCP servers can be registered and discovered
- Dynamic capability loading works
- Integration with existing Elma architecture
- Performance maintained
- `cargo build` passes

## Files to Create/Modify
- `src/mcp_client.rs` (new)
- `src/tool_discovery.rs` (modify)
- `src/tools.rs` (modify)
- MCP configuration (new)
- Documentation for MCP setup

## Risk Assessment
- LOW: Optional feature, can be disabled
- MCP is emerging standard, good long-term investment
- Backward compatible
- Start with basic implementation