# Task 268: Formal Background Task Management System

## Status: DONE
## Priority: MEDIUM

## Prerequisites
**CRITICAL**: This task depends on Task 073 (Platform Capability Detection) for memory/resource limits. Task 073 must be completed first or this task must include its own resource detection.

## Problem Statement
Elma lacks formal background task management. Claude-Code allows kicking off long-running operations (builds, tests) and continuing with other work while monitoring progress.

## Analysis from Claude-Code
- `TaskCreateTool`, `TaskListTool` for background task management
- Allows main agent to start long-running operations
- Continue with other work while tasks run in background
- Monitor and manage background task progress

## Solution Architecture

### 1. Resource Management (Critical)
- **Memory Limits**: Each background task must have memory limits enforced
- **OS Integration**: Use cgroups (Linux) or process limits (macOS/Windows)
- **Pre-flight Check**: Verify available memory before starting task
- **Runtime Monitoring**: Track memory usage during execution
- **Auto-kill Threshold**: Terminate task if memory exceeds limit
- **Configuration**:
  ```toml
  [background_tasks]
  max_concurrent = 3
  default_memory_limit_mb = 2048
  default_timeout_seconds = 300
  ```

### 2. UI Integration (Critical)
- **Sub-terminal Panel**: Use ratatui to show background task output in real-time
- **Not a separate terminal**: Rendered within main Elma TUI
- **Visual Elements**:
  - Task list panel showing all background tasks
  - Per-task: name, status (running/completed/failed), memory usage, runtime
  - Toggleable detail view showing live stdout/stderr
  - Progress indicator for long operations
- **Implementation**: Extend existing `src/ui/ui_terminal.rs` with background task panel

### 3. Task Manager
- Create `src/background_task.rs` for task coordination
- Use tokio tasks for background work
- Status tracking: pending, running, completed, failed, cancelled
- Progress monitoring with result aggregation

## Implementation Steps

### Phase 1: Resource Detection (Prerequisite)
1. Create lightweight resource detection in `src/background_task.rs`
   - Get available system memory
   - Detect platform-specific process limits
   - Reuse Task 073 logic if available

### Phase 2: Task Framework
1. Create BackgroundTask struct with:
   - `id`, `name`, `command`, `workdir`
   - `memory_limit_mb`, `timeout_seconds`
   - `status`, `exit_code`, `started_at`, `memory_usage_mb`
   - `stdout_buffer`, `stderr_buffer`
2. Implement task creation, monitoring, cancellation
3. Add async execution with tokio
4. Memory enforcement during execution

### Phase 3: UI Integration
1. Add background tasks panel to `src/ui/ui_terminal.rs`
2. Show task list in main view (collapsible panel)
3. Live output streaming to sub-panel
4. Keyboard shortcuts:
   - `Ctrl+T` toggle background tasks panel
   - `Enter` view task details
   - `Ctrl+C` cancel selected task

### Phase 4: Integration
1. Wire into orchestration_loop.rs
2. Support background execution in streaming_tool_executor.rs
3. Add tool definitions for background task control

## Integration Points
- `src/background_task.rs`: New module for task management
- `src/ui/ui_terminal.rs`: Add background tasks panel (ratatui)
- `src/orchestration_loop.rs`: Integrate background task handling
- `src/execution.rs`: Support background execution modes
- `src/streaming_tool_executor.rs`: Background execution support

## UI Mockup
```
┌─────────────────────────────────────────────────────────────┐
│ > Show me the project structure                              │
│ ∴ Thinking                                                   │
├─────────────────────────────────────────────────────────────┤
│ ● Running `find . -type f -name "*.rs" | head -50`           │
│   Memory: 45MB | Runtime: 2s | [View Output] [Cancel]       │
├─────────────────────────────────────────────────────────────┤
│ ● Project structure completed                                │
└─────────────────────────────────────────────────────────────┘
```

## Success Criteria
- [x] Background tasks run without blocking main agent
- [x] Memory limits enforced - tasks killed if exceeded
- [x] UI shows real-time task status in ratatui panel (Ctrl+T to toggle)
- [x] Sub-panel shows live stdout/stderr when expanded
- [x] Graceful handling of task failures
- [x] `cargo build` passes

## Files to Create/Modify
- `src/background_task.rs` (new - task manager + resource detection)
- `src/ui/ui_terminal.rs` (modify - add background tasks panel)
- `src/orchestration_loop.rs` (modify)
- `src/execution.rs` (modify)
- `src/streaming_tool_executor.rs` (modify)
- `config/defaults.toml` (add background_tasks section)

## Risk Assessment
- **HIGH**: Memory management is critical - runaway tasks can crash system
- **MEDIUM**: Async task coordination complexity
- **MEDIUM**: UI panel must integrate cleanly with existing ratatui
- Backward compatible feature
- Can be incrementally implemented

## Notes
- This is NOT sub-agent delegation (Task 267) - it's running shell commands in background
- Resource limits are critical - do not skip memory enforcement
- UI must use existing ratatui infrastructure (no new terminal)