# Comprehensive Task: Wire Unused Crates into Elma

## Phase 1: Identify and Catalog All Unused Crates

### Objective
Identify all crates in the elma project that are declared as dependencies but not currently used in the codebase, and plan their integration.

### Steps
1. **Audit declared dependencies** in `Cargo.toml`:
   - `anyhow`, `color-eyre`, `humansize`, `once_cell`, `derive_more`, `itertools`, `tap`
   - `strum`, `console`, `trash`, `clap_mangen`, `serde_path_to_error`
   - `flate2`, `async-trait`, `tokio-stream`, `infer`, `mime_guess`
   - `encoding_rs`, `serde_with`, `ron`, `toml_edit`, `quick-xml`, `comrak`
   - `zip`, `clap`, `clap_complete`, `miette`, `reqwest`, `serde`
   - `serde_json`, `directories`, `dialoguer`, `thiserror`, `tempfile`
   - `shlex`, `url`, `toml`, `tracing`, `tracing-subscriber`
   - `jsonrepair-rs`, `ignore`, `futures`, `which`, `crossterm`
   - `ratatui`, `unicode-width`, `indicatif`, `inquire`, `syntect`
   - `pulldown-cmark`, `similar`, `pdf-extract`, `epub`, `html2text`
   - `djvu-rs`, `mobi`, `regex`, `portable-pty`, `strip-ansi-escapes`

2. **Identify unused crates** by searching for actual usage:
   - Check for `use` statements referencing each crate
   - Verify if crates are only imported but never instantiated
   - Document which crates are truly unused vs. used in specific modules

3. **Create a usage map**:
   ```
   Used Crates (examples):
   - clap: Used in main.rs for CLI argument parsing
   - tokio: Used in main.rs for async runtime
   - serde: Used throughout for serialization
   - directories: Used for path management
   
   Potentially Unused (verify):
   - pdf-extract: Check if PDF tool integration exists
   - epub: Check if EPUB tool integration exists
   - djvu-rs: Check if DjVu tool integration exists
   ```

## Phase 2: Create Tools Discovery Module

### Objective
Establish a comprehensive tool discovery and registry system that can:
- Scan the system for available CLI tools
- Register built-in Elma tools
- Provide tool metadata for UI display
- Cache discovery results for performance

### Architecture

#### 2.1 Tool Capability Definition (`src/tools/discovery.rs`)
```rust
pub struct ToolCapability {
    pub name: String,
    pub description: String,
    pub command_template: String,
    pub availability: ToolAvailability,
    pub category: ToolCategory,
}

pub enum ToolAvailability {
    AlwaysAvailable,
    ContextDependent(String),
    RequiresPermission,
}

pub enum ToolCategory {
    CliTool,
    ProjectSpecific,
    CustomScript,
    Builtin,
}
```

#### 2.2 Discovery Functions
- `discover_available_tools(workspace: &Path) -> Vec<ToolCapability>`
- `is_executable_script(path: &Path) -> bool`
- `cached_tool_to_capability(cached: &CachedTool) -> ToolCapability`
- `tool_to_cached(tool: &ToolCapability) -> CachedTool`

#### 2.3 Registry Management
- `ToolRegistry` struct with discovered and builtin tools
- `available_tools()` - returns discovered tools
- `builtin_steps()` - returns built-in step definitions
- `describe_tool(name: &str)` - provides tool documentation
- `format_tools_for_prompt()` - generates UI-ready tool list

### Implementation Plan
1. **Complete the discovery module** (513 lines already exist)
2. **Add tool categorization** for different tool types
3. **Implement tool validation** to verify executability
4. **Add caching mechanism** with 7-day TTL
5. **Create tool metadata** for documentation

## Phase 3: Wire Tool Registry into Application Flow

### Objective
Integrate the tool registry into the main application flow so tools are:
- Discovered at startup
- Available for command execution
- Displayed in the help system
- Accessible via slash commands

### Integration Points

#### 3.1 Main Application (`src/main.rs`)
```rust
// Add tool registry to AppRuntime
pub struct AppRuntime {
    // ... existing fields ...
    pub tool_registry: tools::ToolRegistry,
}

impl AppRuntime {
    pub fn new() -> Result<Self> {
        let tool_registry = tools::ToolRegistry::new(&workspace_path);
        // ... existing initialization ...
    }
}
```

#### 3.2 Command Handlers (`src/app_chat_handlers.rs`)
```rust
/// Handle /tools command - show available tools
pub(crate) fn handle_tools(runtime: &mut AppRuntime) -> Result<()> {
    let tools_list = runtime.tool_registry.format_tools_for_prompt();
    tui.add_message(MessageRole::Assistant, tools_list);
    Ok(())
}

/// Handle /describe <tool> command
pub(crate) fn handle_describe(runtime: &mut AppRuntime, tool_name: &str) -> Result<()> {
    if let Some(description) = runtime.tool_registry.describe_tool(tool_name) {
        tui.add_message(MessageRole::Assistant, description);
    } else {
        tui.add_error(format!("Tool '{}' not found", tool_name));
    }
    Ok(())
}
```

#### 3.3 Chat Loop (`src/app_chat_loop.rs`)
```rust
fn try_workspace_discovery(runtime: &mut AppRuntime, line: &str) {
    // ... existing implementation ...
}

// Add tool invocation in command routing
match command {
    "/tools" => handle_tools(runtime)?,
    "/describe <tool>" => handle_describe(runtime, args)?,
    // ... other commands ...
}
```

#### 3.4 Tool Execution Pipeline
```rust
// In orchestration or execution modules
pub fn execute_tool(
    runtime: &AppRuntime,
    tool_name: &str,
    args: &str,
) -> Result<ExecutionResult> {
    // Find tool capability
    let tool_cap = runtime.tool_registry
        .available_tools()
        .iter()
        .find(|t| t.name == tool_name)
        .ok_or_else(|| anyhow::anyhow!("Tool not found: {}", tool_name))?;
    
    // Check availability
    if !is_tool_available(tool_cap, runtime) {
        return Err(anyhow::anyhow!("Tool not available: {}", tool_name));
    }
    
    // Execute tool
    execute_command(&tool_cap.command_template, args, runtime)
}
```

## Phase 4: Add CLI Tool Commands

### Objective
Create dedicated CLI commands for tool management and execution.

### New Slash Commands
1. **`/tools`** - List all available tools
2. **`/describe <tool>`** - Show tool description
3. **`/run <tool> [args...]`** - Execute a tool directly
4. **`/tools refresh`** - Force re-scan for tools
5. **`/tools cache`** - Show cache status

### Implementation
```rust
// In command router
"/tools" => handle_tools(runtime)?,
"/describe" => handle_describe(runtime, args)?,
"/run" => handle_run_tool(runtime, args)?,
"/tools refresh" => handle_refresh_tools(runtime)?,
"/tools cache" => handle_cache_status(runtime)?,
```

## Phase 5: Implement Tool Execution and Result Handling

### Objective
Create infrastructure for executing tools and handling their output.

### Components

#### 5.1 Tool Execution Engine
```rust
pub fn execute_tool_command(
    template: &str,
    args: &str,
    runtime: &AppRuntime,
) -> Result<String> {
    // Parse command template
    // Substitute placeholders with actual args
    // Execute via portable-pty
    // Capture and return output
}
```

#### 5.2 Result Handling
```rust
pub struct ExecutionResult {
    pub output: String,
    pub success: bool,
    pub tool_name: String,
    pub duration: Duration,
}

impl ToolResultProcessor {
    pub fn process(&self, result: ExecutionResult) -> Result<()> {
        // Parse tool output
        // Apply any transformations
        // Store in tool_result_storage
        // Update UI
    }
}
```

#### 5.3 Async Execution
```rust
pub async fn execute_tool_async(
    tool: &ToolCapability,
    args: &[String],
    runtime: &AppRuntime,
) -> Result<ExecutionResult> {
    // Use tokio for async execution
    // Handle timeouts
    // Stream output in real-time
}
```

## Phase 6: Add UI Components

### Objective
Create user interface elements for tool discovery and selection.

### UI Components

#### 6.1 Tool Discovery Modal
- List all available tools with descriptions
- Filter/search functionality
- Category tabs (CLI, Project-specific, Scripts, Builtin)
- Status indicators (available/requires-permission/context-dependent)

#### 6.2 Tool Execution Panel
- Command builder interface
- Argument input fields
- Real-time output display
- Success/error indicators

#### 6.3 Integration with Existing UI
```rust
// In ui components
pub struct ToolDiscoveryUI {
    registry: ToolRegistry,
    filter: String,
    selected_category: ToolCategory,
}

impl UITool for ToolDiscoveryUI {
    fn render(&self) -> String {
        // Generate markdown/UI for tool list
    }
    
    fn handle_input(&mut self, input: &str) {
        // Handle user selection
    }
}
```

## Phase 7: Create Tests

### Objective
Ensure tool discovery and execution work correctly.

### Test Categories

#### 7.1 Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_tool_discovery_basic() {
        let workspace = Path::new(".");
        let tools = discover_available_tools(workspace);
        assert!(!tools.is_empty());
    }
    
    #[test]
    fn test_tool_categorization() {
        let tool = ToolCapability {
            name: "test-tool".to_string(),
            // ...
        };
        assert_eq!(tool.category, ToolCategory::CliTool);
    }
}
```

#### 7.2 Integration Tests
```rust
#[tokio::test]
async fn test_tool_execution() {
    let runtime = setup_test_runtime().await;
    let result = execute_tool_command("echo", &["hello"], &runtime).await;
    assert!(result.is_ok());
    assert!(result.unwrap().contains("hello"));
}
```

#### 7.3 UI Tests
```rust
#[test]
fn test_tool_list_rendering() {
    let registry = ToolRegistry::new(&test_workspace());
    let ui = ToolDiscoveryUI::new(registry);
    let rendered = ui.render();
    assert!(rendered.contains("Available Tools"));
}
```

## Phase 8: Documentation

### Objective
Document the tool system architecture and integration patterns.

### Documentation Structure

#### 8.1 Architecture Document
- High-level overview of tool system
- Component interactions diagram
- Data flow diagrams
- Caching strategy explanation

#### 8.2 API Documentation
```rust
/// Tool Registry - Central tool management system
/// 
/// # Examples
/// ```
/// let registry = ToolRegistry::new(&workspace_path);
/// let tools = registry.available_tools();
/// ```
pub struct ToolRegistry { ... }
```

#### 8.3 Developer Guide
- How to add new tool integrations
- Tool capability specification
- Testing guidelines
- Performance considerations

## Performance Considerations

1. **Caching**: Discovery results cached for 7 days
2. **Async Execution**: Tools execute asynchronously to prevent UI blocking
3. **Lazy Loading**: Tool details loaded on-demand
4. **Parallel Discovery**: Scan multiple tool locations concurrently

## Security Considerations

1. **Path Validation**: All paths validated against workspace root
2. **Permission Checks**: Tools requiring special permissions are flagged
3. **Command Injection Prevention**: Proper argument escaping
4. **Sandboxing**: Tools execute in controlled environments

## Migration Path

1. **Backward Compatibility**: Existing commands remain functional
2. **Gradual Rollout**: Enable tool system incrementally
3. **User Feedback**: Collect feedback on tool usability
4. **Iterative Improvements**: Refine based on usage patterns

## Success Criteria

- [ ] All declared dependencies are properly utilized
- [ ] Tool discovery works across different environments
- [ ] Tool execution is reliable and handles errors gracefully
- [ ] UI components render correctly and are responsive
- [ ] Performance is acceptable (< 2s for tool discovery)
- [ ] All tests pass
- [ ] Documentation is complete and accurate