# Task 015: Autonomous Tool Discovery

## Status
PENDING

## Problem
Elma only uses pre-defined step types (shell, edit, select, etc.). It can't discover new tools or compose them creatively.

## Goal
Add a tool discovery mechanism that scans for available capabilities and makes them available to the orchestrator.

## Implementation Steps

1. **Create tool discovery module** `src/tools/discovery.rs`:
   ```rust
   pub struct ToolCapability {
       pub name: String,
       pub description: String,
       pub command_template: String,
       pub availability: ToolAvailability,
   }
   
   pub enum ToolAvailability {
       AlwaysAvailable,
       ContextDependent(String),  // e.g., "only in Rust projects"
       RequiresPermission,
   }
   
   pub async fn discover_available_tools(workspace: &Path) -> Vec<ToolCapability>;
   ```

2. **Scan for tool categories**:
   - **CLI tools**: git, rg, find, jq, curl, ssh, etc.
   - **Project-specific**: cargo (Rust), npm (JS), pip (Python), etc.
   - **Custom scripts**: Executable files in repo (.sh, .py, etc.)
   - **API endpoints**: From config files

3. **Create tool registry** `src/tools/registry.rs`:
   ```rust
   pub struct ToolRegistry {
       discovered: Vec<ToolCapability>,
       builtin: Vec<BuiltinStep>,
   }
   
   impl ToolRegistry {
       pub fn available_tools(&self, context: &ExecutionContext) -> Vec<&ToolCapability>;
       pub fn describe_tool(&self, name: &str) -> Option<String>;
   }
   ```

4. **Update orchestrator to use tool registry**:
   ```rust
   // In orchestration prompt
   Available Tools:
   - git: Version control operations
   - rg: Fast text search
   - cargo: Rust package manager (detected: Cargo.toml present)
   - ./scripts/deploy.sh: Custom deployment script
   
   You can use these tools in shell steps or request new tool compositions.
   ```

5. **Add tool composition capability**:
   ```rust
   // Allow model to compose multi-tool operations
   pub struct ToolComposition {
       pub tools: Vec<String>,
       pub pipeline: String,  // e.g., "rg 'pattern' | jq '.field'"
   }
   ```

6. **Add tool validation**:
   - Check tool exists before execution
   - Validate command safety
   - Log tool usage for learning

## Acceptance Criteria
- [ ] Tool discovery scans workspace on startup
- [ ] Available tools are passed to orchestrator
- [ ] Project-specific tools are detected (cargo, npm, etc.)
- [ ] Custom scripts are discovered
- [ ] Tool usage is logged
- [ ] Model can compose multiple tools

## Files to Create
- `src/tools/discovery.rs` - New module
- `src/tools/registry.rs` - New module
- `src/tools/mod.rs` - Module exports

## Files to Modify
- `src/orchestration.rs` - Use tool registry in prompts
- `src/execution_steps.rs` - Validate discovered tools
- `src/main.rs` - Add tools module

## Priority
MEDIUM - Enhances flexibility but not core reasoning

## Dependencies
- None blocking
