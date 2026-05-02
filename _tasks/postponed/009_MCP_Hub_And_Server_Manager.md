# 142 MCP Hub And Server Manager

## Backlog Reconciliation (2026-05-02)

Superseded by Task 490 for the current extension gateway. Use this only as historical MCP design context.


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
Implement Model Context Protocol (MCP) integration for external tools.

## Reference
- Roo-Code: `~/Roo-Code/src/services/mcp/McpHub.ts`
- Roo-Code: `~/Roo-Code/src/services/mcp/McpServerManager.ts`

## Implementation

### 1. MCP Types
File: `src/mcp/types.rs` (new)
- `McpServer` - server connection config
- `McpTool` - tool definition from MCP
- `McpRequest` - tool call request
- `McpResponse` - tool call response

### 2. MCP Server Manager
File: `src/mcp/server_manager.rs` (new)
- `McpServerManager` - manages server connections
- `add_server(config)` - add MCP server
- `remove_server(name)` - remove server
- `list_servers()` - list configured servers
- `start_server(name)` / `stop_server(name)`

### 3. MCP Hub
File: `src/mcp/hub.rs` (new)
- `McpHub` - routes tool calls to MCP servers
- `discover_tools()` - fetch tools from all servers
- `call_tool(server, tool, args)` - execute tool call
- Integrate with tool calling pipeline

### 4. Config Format
File: `config/mcp_servers.toml`
```toml
[[servers]]
name = "filesystem"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/path/to/workspace"]

[[servers]]
name = "brave-search"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-brave-search"]
env = { BRAVE_API_KEY = "..." }
```

### 5. UseMcpTool Integration
File: `src/tools/use_mcp.rs` (new)
- Tool name: `use_mcp`
- Route MCP tool calls through hub

## Verification
- [ ] `cargo build` passes
- [ ] MCP servers can be configured
- [ ] Tool calls route to MCP servers