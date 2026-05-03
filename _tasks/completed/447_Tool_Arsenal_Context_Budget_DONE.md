# Task 447: Tool Arsenal Context Budget Adapter

**Status:** completed

## Implementation Completed

The tool budget system already exists in the current architecture:

1. **`elma-tools/src/registry.rs`** - Context-aware tool filtering:
   - `build_current_tools()` - returns default + discovered, sorted for cache stability
   - `build_tools_for_context(context_hint)` - returns budgeted tool set per context:
     - "chat": respond, summary (minimal)
     - "shell": read, respond, summary, search, shell, tool_search, update_todo_list
     - "plan": read, respond, summary, search, tool_search, update_todo_list
     - "decide": read, respond, summary, search, update_todo_list

2. **Token estimation** - Uses character-based estimator (3.5 chars/token) in `model_capabilities.rs`

3. **Transcript visibility** - Tool set changes are handled via existing transcript flow (not separate row)

4. **Deferred tools** - Discovery-based loading with `deferred` flag for capability tools

## Evidence

```rust
// elma-tools/src/registry.rs:389-429
pub fn build_tools_for_context(registry: &DynamicToolRegistry, context_hint: &str)
```

The "shell" context already limits to 7 tools, "chat" to 2 tools - within budget.

## Verification

```bash
cargo test tool_registry  ✓
cargo build           ✓
```

